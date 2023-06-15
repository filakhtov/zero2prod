use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

#[tracing::instrument(name = "Persisting subscriber", skip(new_subscriber, db_connection))]
async fn insert_subscriber(
    new_subscriber: &NewSubscriber,
    db_connection: &MySqlPool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO `subscriptions` (`id`, `email`, `name`, `subscribed_at`)
        VALUES (?, ?, ?, ?)
        "#,
        Uuid::new_v4().to_string(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
    )
    .execute(db_connection)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute the query: {:?}", e);

        e
    })?;
    Ok({})
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, db_connection),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    db_connection: web::Data<MySqlPool>,
) -> impl Responder {
    let name = match SubscriberName::parse(&form.0.name) {
        Ok(name) => name,
        _ => return HttpResponse::BadRequest(),
    };
    let email = match SubscriberEmail::parse(&form.0.email) {
        Ok(email) => email,
        _ => return HttpResponse::BadRequest(),
    };
    let new_subscriber = NewSubscriber { email, name };

    match insert_subscriber(&new_subscriber, &db_connection).await {
        Ok(_) => HttpResponse::Ok(),
        _ => HttpResponse::InternalServerError(),
    }
}
