use actix_web::{web, HttpResponse, Responder};
use sqlx::MySqlPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(
    name = "Get subscriber_id using token",
    skip(subscription_token, db_pool)
)]
async fn get_subscriber_id_from_token(
    db_pool: &MySqlPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT `subscriber_id` FROM `subscription_tokens` WHERE `subscription_token`=?"#,
        subscription_token
    )
    .fetch_optional(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    let subscriber_id = match result {
        Some(subscription) => match Uuid::parse_str(&subscription.subscriber_id) {
            Ok(id) => Some(id),
            Err(e) => {
                tracing::error!("Failed to parse UUID received from db: {:?}", e);
                None
            }
        },
        _ => None,
    };

    Ok(subscriber_id)
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(db_pool, subscriber_id))]
async fn confirm_subscriber(db_pool: &MySqlPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE `subscriptions` SET `status`='confirmed' WHERE `id`=?"#,
        subscriber_id.to_string(),
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
) -> impl Responder {
    let id = match get_subscriber_id_from_token(&db_pool, &parameters.subscription_token).await {
        Ok(id) => id,
        _ => return HttpResponse::InternalServerError().finish(),
    };
    match id {
        Some(subscriber_id) => {
            if confirm_subscriber(&db_pool, subscriber_id).await.is_err() {
                return HttpResponse::InternalServerError().finish();
            }
        }
        _ => return HttpResponse::NotFound().finish(),
    }

    HttpResponse::Ok().finish()
}
