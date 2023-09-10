use actix_web::{http::StatusCode, web, HttpResponse, Responder, ResponseError};
use anyhow::Context;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::errors::error_chain_fmt;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[derive(thiserror::Error)]
#[error(transparent)]
struct GetSubscriberError(#[from] anyhow::Error);

impl std::fmt::Debug for GetSubscriberError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[derive(thiserror::Error)]
#[error(transparent)]
pub struct ConfirmSubscriptionError(#[from] anyhow::Error);

impl std::fmt::Debug for ConfirmSubscriptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for ConfirmSubscriptionError {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

#[tracing::instrument(
    name = "Get subscriber_id using token",
    skip(subscription_token, db_pool)
)]
async fn get_subscriber_id_from_token(
    db_pool: &MySqlPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, GetSubscriberError> {
    let result = sqlx::query!(
        r#"SELECT `subscriber_id` FROM `subscription_tokens` WHERE `subscription_token`=?"#,
        subscription_token
    )
    .fetch_optional(db_pool)
    .await
    .context("Failed to execute query")?;

    let subscription = match result {
        Some(subscription) => subscription,
        _ => return Ok(None),
    };

    let subscriber_id = Uuid::parse_str(&subscription.subscriber_id)
        .context("Failed to parse UUID obtained from the database.")?;

    Ok(Some(subscriber_id))
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(db_pool, subscriber_id))]
async fn confirm_subscriber(db_pool: &MySqlPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE `subscriptions` SET `status`='confirmed' WHERE `id`=?"#,
        subscriber_id,
    )
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}

#[tracing::instrument(name = "Confirm a pending subscription", skip(parameters, db_pool))]
pub async fn confirm(
    parameters: web::Query<Parameters>,
    db_pool: web::Data<MySqlPool>,
) -> Result<impl Responder, ConfirmSubscriptionError> {
    let id = get_subscriber_id_from_token(&db_pool, &parameters.subscription_token)
        .await
        .context("Unable to fetch a subscriber token from the database.")?;
    match id {
        Some(subscriber_id) => {
            confirm_subscriber(&db_pool, subscriber_id)
                .await
                .context("Failed to confirm subscriber.")?;

            Ok(HttpResponse::Ok().finish())
        }
        _ => Ok(HttpResponse::NotFound().finish()),
    }
}
