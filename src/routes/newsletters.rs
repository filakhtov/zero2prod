use actix_web::{HttpResponse, Responder};

pub async fn publish_newsletter() -> impl Responder {
    HttpResponse::Ok().finish()
}
