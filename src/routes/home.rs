use actix_web::{http::header::ContentType, HttpResponse, Responder};

pub async fn home() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(include_str!("home.html"))
}
