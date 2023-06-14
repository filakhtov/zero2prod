use sqlx::mysql::MySqlPoolOptions;
use std::{net::TcpListener, time::Duration};
use zero2prod::{
    configuration::get_configuration,
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

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration(&get_configuration_path())
        .expect("Failed to read the `{}` configuration file");
    let db_connection_pool = MySqlPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(configuration.database.with_db());
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address)?;
    run(listener, db_connection_pool)?.await
}
