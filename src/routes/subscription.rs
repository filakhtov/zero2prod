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
        Ok(_) => HttpResponse::Ok(),
        Err(e) => {
            println!("Failed to execute the query: {}", e);
            HttpResponse::InternalServerError()
        }
    }
}
