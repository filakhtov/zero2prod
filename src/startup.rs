use crate::routes::{health_check, subscribe};
use actix_web::{dev::Server, web, App, HttpServer};
use sqlx::MySqlPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub fn run(listener: TcpListener, db_connection_pool: MySqlPool) -> Result<Server, std::io::Error> {
    let connection = web::Data::new(db_connection_pool);
    Ok(HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(connection.clone())
    })
    .listen(listener)?
    .run())
}
