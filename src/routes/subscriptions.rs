use actix_web::{http::StatusCode, web, HttpResponse, Responder, ResponseError};
use anyhow::Context;
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sqlx::{MySql, MySqlPool, Transaction};
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    errors::error_chain_fmt,
    startup::ApplicationBaseUrl,
};

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(&form.name)?;
        let email = SubscriberEmail::parse(&form.email)?;
        Ok(NewSubscriber { name, email })
    }
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    Validation(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Validation(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Persisting subscriber", skip(new_subscriber, db_transaction))]
async fn persist_subscriber(
    db_transaction: &mut Transaction<'_, MySql>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO `subscriptions` (`id`, `email`, `name`, `subscribed_at`, `status`)
        VALUES (?, ?, ?, ?, "pending_confirmation")
        "#,
        subscriber_id.to_string(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
    )
    .execute(db_transaction)
    .await?;

    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url, subscription_token)
)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token,
    );
    let plain_body = format!(
        "Welcome to our newsletter\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    let html_body = format!(
        "Welcome to our newsletter!<br />\
                Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email(&new_subscriber.email, "Welcome", &html_body, &plain_body)
        .await?;

    Ok(())
}

fn generate_subscription_token() -> String {
    let rng = thread_rng();
    rng.sample_iter(Alphanumeric)
        .map(char::from)
        .take(25)
        .collect()
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, db_transaction)
)]
async fn persist_token(
    db_transaction: &mut Transaction<'_, MySql>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO `subscription_tokens` (`subscription_token`, `subscriber_id`) VALUES (?, ?)"#,
        subscription_token,
        subscriber_id.to_string(),
    ).execute(db_transaction).await?;

    Ok(())
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, db_pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    db_pool: web::Data<MySqlPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<impl Responder, SubscribeError> {
    let new_subscriber = form.0.try_into().map_err(SubscribeError::Validation)?;
    let mut db_transaction = db_pool
        .begin()
        .await
        .context("Failed to acquire a database connection from the pool.")?;
    let subscriber_id = persist_subscriber(&mut db_transaction, &new_subscriber)
        .await
        .context("Failed to insert new subscriber in the database.")?;
    let subscription_token = &generate_subscription_token();
    persist_token(&mut db_transaction, subscriber_id, subscription_token)
        .await
        .context("Failed to store the confirmation token for a new subscriber.")?;
    db_transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction.")?;
    send_confirmation_email(
        email_client.as_ref(),
        new_subscriber,
        &base_url.0,
        subscription_token,
    )
    .await
    .context("Failed to send a confirmation email.")?;

    Ok(HttpResponse::Ok().finish())
}
