use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use sqlx::MySqlPool;
use tracing::Instrument;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

pub async fn subscribe(
    form: web::Form<FormData>,
    db_connection: web::Data<MySqlPool>,
) -> impl Responder {
    let request_id = Uuid::new_v4();

    let request_span = tracing::info_span!(
        "Adding a new subscriber",
        %request_id,
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    );
    let _request_span_guard = request_span.enter();

    tracing::info!("Adding a new subscriber");

    let query_span = tracing::info_span!("Persisting a new subscriber");

    match sqlx::query!(
        r#"
        INSERT INTO `subscriptions` (`id`, `email`, `name`, `subscribed_at`)
        VALUES (?, ?, ?, ?)
        "#,
        Uuid::new_v4().to_string(),
        form.email,
        form.name,
        Utc::now(),
    )
    .execute(db_connection.get_ref())
    .instrument(query_span)
    .await
    {
        Ok(_) => {
            tracing::info!("{} new subscriber details have been saved", request_id);
            HttpResponse::Ok()
        }
        Err(e) => {
            tracing::error!("{} failed to execute the query: {:?}", e, request_id);
            HttpResponse::InternalServerError()
        }
    }
}
