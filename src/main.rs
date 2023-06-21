use actix_web::dev::Server;
use sqlx::mysql::MySqlPoolOptions;
use std::{net::TcpListener, time::Duration};
use zero2prod::{
    configuration::{get_configuration, Settings},
    email_client::EmailClient,
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};

fn get_configuration_path() -> String {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1) {
        Some(path) => path.to_owned(),
        None => {
            eprintln!("usage: {} <configuration_file>", args[0]);
            std::process::exit(1);
        }
    }
}

async fn build(configuration: Settings) -> Result<Server, std::io::Error> {
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

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration(&get_configuration_path())
        .expect("Failed to read the `{}` configuration file");

    let server = build(configuration).await?;
    server.await?;

    Ok(())
}
