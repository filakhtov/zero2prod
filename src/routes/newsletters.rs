use actix_web::{http::header, web, HttpRequest, HttpResponse, Responder, ResponseError};
use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base64::Engine;
use secrecy::{ExposeSecret, Secret};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{domain::SubscriberEmail, email_client::EmailClient, errors::error_chain_fmt};

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[derive(thiserror::Error)]
#[error(transparent)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse {
        match self {
            Self::AuthError(_) => HttpResponse::Unauthorized()
                .insert_header((header::WWW_AUTHENTICATE, r#"Basic realm="publish""#))
                .finish(),
            _ => HttpResponse::InternalServerError().finish(),
        }
    }
}

struct Credentials {
    username: String,
    password: Secret<String>,
}

fn basic_authentication(headers: &header::HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The `Authorization` header was missing.")?
        .to_str()
        .context("The `Authorization` is not a valid UTF-8 string.")?;

    let base64_encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme is not `Basic`.")?;

    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_encoded_segment)
        .context("Failed to base64-decode `Basic` credentials.")?;

    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not a valid UTF-8.")?;

    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided for `Basic` auth."))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided for `Basic` auth."))?
        .to_string();

    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}

#[tracing::instrument(name = "Load stored credentials", skip(username, db_pool))]
async fn load_stored_credentials(
    username: &str,
    db_pool: &MySqlPool,
) -> Result<Option<(Uuid, Secret<String>)>, anyhow::Error> {
    let result = sqlx::query!(
        r#"
            SELECT `id`, `password_hash`
              FROM `users`
             WHERE `username`=?
        "#,
        username,
    )
    .fetch_optional(db_pool)
    .await
    .context("Load stored credentials query failed.")?;

    match result {
        Some(row) => {
            let uuid = Uuid::parse_str(&row.id)
                .context("Failed to parse user UUID loaded from the database.")?;
            let password = Secret::new(row.password_hash);
            Ok(Some((uuid, password)))
        }
        _ => Ok(None),
    }
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, db_pool))]
async fn validate_credentials(
    credentials: Credentials,
    db_pool: &MySqlPool,
) -> Result<Uuid, PublishError> {
    let (user_id, expected_password_hash) = load_stored_credentials(&credentials.username, db_pool)
        .await
        .map_err(PublishError::UnexpectedError)?
        .ok_or_else(|| PublishError::AuthError(anyhow::anyhow!("Unknown username.")))?;

    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse a PHC formatted string hash loaded from the database.")
        .map_err(PublishError::UnexpectedError)?;

    tracing::info_span!("Verify password hash")
        .in_scope(|| {
            Argon2::default().verify_password(
                credentials.password.expose_secret().as_bytes(),
                &expected_password_hash,
            )
        })
        .context("Invalid password")
        .map_err(PublishError::AuthError)?;

    Ok(user_id)
}

#[tracing::instrument(
    name = "Publish a newsletter",
    skip(body, pool, email_client, request),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    pool: web::Data<MySqlPool>,
    body: web::Json<BodyData>,
    email_client: web::Data<EmailClient>,
    request: HttpRequest,
) -> Result<impl Responder, PublishError> {
    let credentials = basic_authentication(request.headers()).map_err(PublishError::AuthError)?;
    let current_span = tracing::Span::current();
    current_span.record("username", &tracing::field::display(&credentials.username));

    let user_id = validate_credentials(credentials, &pool).await?;
    current_span.record("user_id", &tracing::field::display(&user_id));

    let subscribers = get_confirmed_subscribers(&pool).await?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send a newseletter issue to {}", subscriber.email)
                    })?;
            }
            Err(error) => {
                tracing::warn!(error.cause_chain = ?error, "Skipping a confirmed subscriber. \
            Their stored email address is invalid.");
            }
        }
    }

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Get a list of confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &MySqlPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let rows = sqlx::query!(
        r#"
            SELECT `email`
            FROM `subscriptions`
            WHERE `status`="confirmed"
        "#,
    )
    .fetch_all(pool)
    .await?;

    let confirmed_subscribers = rows
        .into_iter()
        .map(|row| match SubscriberEmail::parse(&row.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(anyhow::anyhow!(error)),
        })
        .collect();

    Ok(confirmed_subscribers)
}
