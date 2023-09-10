use crate::{
    configuration::Settings, domain::SubscriberEmail, email_client::EmailClient,
    startup::get_connection_pool,
};
use sqlx::MySqlPool;
use std::time::Duration;
use tracing::{field::display, Span};
use uuid::{fmt::Hyphenated, Uuid};

#[tracing::instrument(
    skip_all,
    fields(
        newsletter_issue_id=tracing::field::Empty,
        subscriber_email=tracing::field::Empty,
    )
)]
pub async fn try_execute_task(
    db_pool: &MySqlPool,
    email_client: &EmailClient,
) -> Result<ExecutionOutcome, anyhow::Error> {
    match dequeue_task(db_pool).await? {
        Some((transaction, issue_id, email)) => {
            Span::current()
                .record("newsletter_issue_id", &display(issue_id))
                .record("subscriber_email", &display(&email));

            match SubscriberEmail::parse(&email) {
                Ok(email) => {
                    let issue = get_issue(db_pool, issue_id).await?;
                    if let Err(e) = email_client
                        .send_email(
                            &email,
                            &issue.title,
                            &issue.html_content,
                            &issue.text_content,
                        )
                        .await
                    {
                        tracing::error!(
                            error.cause_chain = %e,
                            error.message = %e,
                            "Failed to deliver issue to a confirmed \
                            subscriber. Skipping."
                        )
                    }
                }
                Err(e) => tracing::error!(
                    error.cause_chain = ?e,
                    error.message = %e,
                    "Skipping a confirmed subscriber. \
                    Their stored email address is invalid",
                ),
            }

            delete_task(transaction, issue_id, &email).await?;

            Ok(ExecutionOutcome::TaskCompleted)
        }
        _ => Ok(ExecutionOutcome::EmptyQueue),
    }
}

type MySqlTransaction = sqlx::Transaction<'static, sqlx::MySql>;

struct IssueQueueItem {
    newsletter_issue_id: Hyphenated,
    subscriber_email: String,
}

#[tracing::instrument(skip_all)]
async fn dequeue_task(
    db_pool: &MySqlPool,
) -> Result<Option<(MySqlTransaction, Uuid, String)>, anyhow::Error> {
    let mut transaction = db_pool.begin().await?;
    let r = sqlx::query_as!(
        IssueQueueItem,
        r#"SELECT `newsletter_issue_id` AS "newsletter_issue_id: Hyphenated", `subscriber_email`
             FROM `issue_delivery_queue`
            LIMIT 1
              FOR UPDATE
             SKIP LOCKED"#
    )
    .fetch_optional(&mut transaction)
    .await?;

    if let Some(r) = r {
        Ok(Some((
            transaction,
            r.newsletter_issue_id.into(),
            r.subscriber_email,
        )))
    } else {
        Ok(None)
    }
}

#[tracing::instrument(skip_all)]
async fn delete_task(
    mut transaction: MySqlTransaction,
    issue_id: Uuid,
    email: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"DELETE FROM `issue_delivery_queue`
            WHERE `newsletter_issue_id` = ? AND `subscriber_email` = ?"#,
        issue_id,
        email
    )
    .execute(&mut transaction)
    .await?;

    transaction.commit().await?;

    Ok(())
}

struct NewsletterIssue {
    title: String,
    text_content: String,
    html_content: String,
}

#[tracing::instrument(skip_all)]
async fn get_issue(db_pool: &MySqlPool, issue_id: Uuid) -> Result<NewsletterIssue, anyhow::Error> {
    let issue = sqlx::query_as!(
        NewsletterIssue,
        r#"SELECT `title`, `text_content`, `html_content`
             FROM `newsletter_issues`
            WHERE `newsletter_issue_id` = ?"#,
        issue_id
    )
    .fetch_one(db_pool)
    .await?;

    Ok(issue)
}

async fn worker_loop(db_pool: MySqlPool, email_client: EmailClient) -> Result<(), anyhow::Error> {
    loop {
        match try_execute_task(&db_pool, &email_client).await {
            Ok(ExecutionOutcome::TaskCompleted) => {}
            Ok(ExecutionOutcome::EmptyQueue) => tokio::time::sleep(Duration::from_secs(10)).await,
            _ => tokio::time::sleep(Duration::from_secs(1)).await,
        }
    }
}

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

pub async fn run_worker_until_stopped(configuration: Settings) -> Result<(), anyhow::Error> {
    let db_pool = get_connection_pool(&configuration.database);
    let email_client = configuration.email.client();

    worker_loop(db_pool, email_client).await
}
