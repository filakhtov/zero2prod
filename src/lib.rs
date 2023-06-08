use actix_web::{dev::Server, web, App, HttpResponse, HttpServer, Responder};
use std::net::TcpListener;

async fn health_check() -> impl Responder {
    HttpResponse::NoContent()
}

pub fn run(listener: TcpListener) -> Result<Server, std::io::Error> {
    Ok(
        HttpServer::new(|| App::new().route("/health_check", web::get().to(health_check)))
            .listen(listener)?
            .run(),
    )
}
