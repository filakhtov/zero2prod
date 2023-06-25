use actix_web::{web, HttpResponse, Responder};

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscription", skip(_parameters))]
pub async fn confirm(_parameters: web::Query<Parameters>) -> impl Responder {
    HttpResponse::Ok().finish()
}
