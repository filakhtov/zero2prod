use actix_web::{http::StatusCode, web, HttpResponse, Responder, ResponseError};
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sqlx::{MySql, MySqlPool, Transaction};
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
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

fn error_chain_fmt<T: std::error::Error + ?Sized>(
    e: &T,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let current = e.source();

    if let Some(cause) = current {
        writeln!(f, "Caused by:\n\t")?;
        error_chain_fmt(cause, f)?;
    }

    Ok(())
}

pub struct PersistTokenError(sqlx::Error);

impl std::fmt::Display for PersistTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while \
            persisting a subscription token."
        )
    }
}

impl std::fmt::Debug for PersistTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl std::error::Error for PersistTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

pub enum SubscribeError {
    Validation(String),
    StoreToken(PersistTokenError),
    SendEmail(reqwest::Error),
    Pool(sqlx::Error),
    InsertSubscriber(sqlx::Error),
    TransactionCommit(sqlx::Error),
}

impl From<reqwest::Error> for SubscribeError {
    fn from(e: reqwest::Error) -> Self {
        Self::SendEmail(e)
    }
}

impl From<PersistTokenError> for SubscribeError {
    fn from(e: PersistTokenError) -> Self {
        Self::StoreToken(e)
    }
}

impl From<String> for SubscribeError {
    fn from(e: String) -> Self {
        Self::Validation(e)
    }
}

impl std::fmt::Display for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(e) => write!(f, "{}", e),
            Self::Pool(_) => write!(f, "Failed to acquire a database connection from the pool."),
            Self::InsertSubscriber(_) => {
                write!(f, "Failed to insert a new subscriber into the database.")
            }
            Self::TransactionCommit(_) => write!(
                f,
                "Failed to commit SQL transaction while saving a new subscriber."
            ),
            Self::StoreToken(_) => write!(
                f,
                "Failed to store the confirmation token for the new subscriber."
            ),
            Self::SendEmail(_) => write!(f, "Failed to send a confirmation email."),
        }
    }
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl std::error::Error for SubscribeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Validation(_) => None,
            Self::Pool(e) => Some(e),
            Self::InsertSubscriber(e) => Some(e),
            Self::TransactionCommit(e) => Some(e),
            Self::StoreToken(e) => Some(e),
            Self::SendEmail(e) => Some(e),
        }
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
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute the query: {:?}", e);

        e
    })?;

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
        .send_email(new_subscriber.email, "Welcome", &html_body, &plain_body)
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
) -> Result<(), PersistTokenError> {
    sqlx::query!(
        r#"INSERT INTO `subscription_tokens` (`subscription_token`, `subscriber_id`) VALUES (?, ?)"#,
        subscription_token,
        subscriber_id.to_string(),
    ).execute(db_transaction).await.map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        PersistTokenError(e)
    })?;

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
    let new_subscriber = form.0.try_into()?;
    let mut db_transaction = db_pool.begin().await.map_err(SubscribeError::Pool)?;
    let subscriber_id = persist_subscriber(&mut db_transaction, &new_subscriber)
        .await
        .map_err(SubscribeError::InsertSubscriber)?;
    let subscription_token = &generate_subscription_token();
    persist_token(&mut db_transaction, subscriber_id, subscription_token).await?;
    db_transaction
        .commit()
        .await
        .map_err(SubscribeError::TransactionCommit)?;
    send_confirmation_email(
        email_client.as_ref(),
        new_subscriber,
        &base_url.0,
        subscription_token,
    )
    .await?;
    Ok(HttpResponse::Ok().finish())
}
