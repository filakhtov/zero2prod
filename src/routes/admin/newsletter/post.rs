use crate::{
    authentication::UserId,
    idempotency::{save_response, try_processing, IdempotencyKey, NextAction},
    utils::{bad_request, internal_server_error, see_other},
};
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::MySqlPool;

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    text_content: String,
    html_content: String,
    idempotency_key: String,
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip_all,
    fields(user_id=%&*user_id)
)]
pub async fn publish_newsletter(
    db_pool: web::Data<MySqlPool>,
    form: web::Form<FormData>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let FormData {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = form.0;
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(bad_request)?;

    if text_content.is_empty() {
        FlashMessage::error("Failed to publish the newsletter: missing text content").send();

        return Ok(see_other("/admin/newsletter"));
    }

    if html_content.is_empty() {
        FlashMessage::error("Failed to publish the newsletter: missing HTML content").send();

        return Ok(see_other("/admin/newsletter"));
    }

    if title.is_empty() {
        FlashMessage::error("Failed to publish the newsletter: missing newsletter title").send();

        return Ok(see_other("/admin/newsletter"));
    }

    let mut transaction = match try_processing(&db_pool, &idempotency_key, *user_id)
        .await
        .map_err(internal_server_error)?
    {
        NextAction::StartProcessing(transaction) => transaction,
        NextAction::ReturnSavedResponse(saved_response) => {
            success_message().send();

            return Ok(saved_response);
        }
    };

    let issue_id = insert_newsletter_issue(&mut transaction, &title, &text_content, &html_content)
        .await
        .context("Failed to store newletter issue details")
        .map_err(internal_server_error)?;
    enqueue_delivery_task(&mut transaction, issue_id)
        .await
        .context("Failed to enqueue delivery tasks")
        .map_err(internal_server_error)?;

    let response = see_other("/admin/newsletter");
    let response = save_response(transaction, &idempotency_key, *user_id, response)
        .await
        .map_err(internal_server_error)?;

    success_message().send();

    Ok(response)
}

fn success_message() -> FlashMessage {
    FlashMessage::info(
        "The newsletter issues has been accepted \
        and emails will be sent out shortly",
    )
}

#[tracing::instrument(skip_all)]
async fn insert_newsletter_issue(
    transaction: &mut sqlx::Transaction<'_, sqlx::MySql>,
    title: &str,
    text_content: &str,
    html_content: &str,
) -> Result<uuid::Uuid, sqlx::Error> {
    let newsletter_issue_id = uuid::Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO `newsletter_issues` (
            `newsletter_issue_id`, `title`, `text_content`, `html_content`, `published_at`
        ) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP())"#,
        newsletter_issue_id,
        title,
        text_content,
        html_content,
    )
    .execute(transaction)
    .await?;

    Ok(newsletter_issue_id)
}

#[tracing::instrument(skip_all)]
async fn enqueue_delivery_task(
    transaction: &mut sqlx::Transaction<'_, sqlx::MySql>,
    newsletter_issue_id: uuid::Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO `issue_delivery_queue` (
            `newsletter_issue_id`, `subscriber_email`
        ) SELECT ?, `email` FROM `subscriptions`
           WHERE `status`="confirmed""#,
        newsletter_issue_id,
    )
    .execute(transaction)
    .await?;

    Ok(())
}
