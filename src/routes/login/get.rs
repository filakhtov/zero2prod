use actix_web::{http::header::ContentType, HttpResponse, Responder};

pub async fn login_form() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(include_str!("login.html"))
}
