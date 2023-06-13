use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use sqlx::MySqlPool;
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

    log::info!(
        "{} Adding {} <{}> as a new subscriber",
        request_id,
        form.name,
        form.email
    );

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
    .await
    {
        Ok(_) => {
            log::info!("{} new subscriber details have been saved", request_id);
            HttpResponse::Ok()
        }
        Err(e) => {
            log::error!("{} failed to execute the query: {:?}", e, request_id);
            HttpResponse::InternalServerError()
        }
    }
}
