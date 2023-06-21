use crate::{
    configuration::Settings,
    email_client::EmailClient,
    routes::{health_check, subscribe},
};
use actix_web::{dev::Server, web, App, HttpServer};
use sqlx::{mysql::MySqlPoolOptions, MySqlPool};
use std::{net::TcpListener, time::Duration};
use tracing_actix_web::TracingLogger;

pub async fn build(configuration: Settings) -> Result<Server, std::io::Error> {
    let db_connection_pool = MySqlPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(configuration.database.with_db());

    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address)?;
    let timeout = configuration.email.timeout();

    let sender_email = configuration
        .email
        .sender()
        .expect("Invalid sender email address");
    let email_client = EmailClient::new(
        configuration.email.base_url,
        sender_email,
        configuration.email.authorization_token,
        timeout,
    );

    run(listener, db_connection_pool, email_client)
}

pub fn run(
    listener: TcpListener,
    db_connection_pool: MySqlPool,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    let connection = web::Data::new(db_connection_pool);
    let email_client = web::Data::new(email_client);
    Ok(HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(connection.clone())
            .app_data(email_client.clone())
    })
    .listen(listener)?
    .run())
}
