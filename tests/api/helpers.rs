use sqlx::{Executor, MySqlPool};
use std::net::TcpListener;
use tokio::sync::OnceCell;
use uuid::Uuid;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    email_client::EmailClient,
    telemetry::{get_subscriber, init_subscriber},
};
pub struct TestApp {
    pub address: String,
    pub db_pool: MySqlPool,
}

async fn cleanup_database(db_settings: &DatabaseSettings) {
    let db_connection = MySqlPool::connect_with(db_settings.without_db())
        .await
        .expect("Failed to connect to the database");
    let rows = sqlx::query!(
        r#"
        SELECT DISTINCT `table_schema`
        FROM `information_schema`.`tables`
        WHERE `table_schema` LIKE 'newsletter_%'
        "#
    )
    .fetch_all(&db_connection)
    .await
    .expect("Failed to select previous test database schemas");

    for row in rows {
        db_connection
            .execute(format!(r#"DROP DATABASE `{}`"#, row.table_schema).as_str())
            .await
            .expect("Failed to delete old test database schema");
    }
}

async fn create_database(db_settings: &DatabaseSettings) {
    let db_connection = MySqlPool::connect_with(db_settings.without_db())
        .await
        .expect("Failed to connect to the database");
    db_connection
        .execute(format!(r#"CREATE DATABASE `{}`;"#, db_settings.database_name).as_str())
        .await
        .expect("Failed to create database");
}

async fn migrate_database(db_settings: &DatabaseSettings) -> MySqlPool {
    let db_pool = MySqlPool::connect_with(db_settings.with_db())
        .await
        .expect("Failed to connect to the database");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to run migrations");

    db_pool
}

static TEST_SETUP: OnceCell<()> = OnceCell::const_new();

fn should_display_output() -> bool {
    std::env::args().any(|a| a == *"--nocapture")
}

pub async fn spawn_app() -> TestApp {
    let mut configuration =
        get_configuration("test.yaml").expect("Failed to read test configuration");

    TEST_SETUP
        .get_or_init(|| async {
            let name = "test".into();
            let env_filter = "debug".into();

            match should_display_output() {
                true => {
                    init_subscriber(get_subscriber(name, env_filter, std::io::stdout));
                }
                _ => {
                    init_subscriber(get_subscriber(name, env_filter, std::io::sink));
                }
            };

            cleanup_database(&configuration.database).await;
        })
        .await;

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to the random port");
    let port = listener.local_addr().unwrap().port();

    configuration.database.database_name = format!(
        "{}_{}",
        configuration.database.database_name,
        Uuid::new_v4().as_simple(),
    );
    create_database(&configuration.database).await;
    let db_pool = migrate_database(&configuration.database).await;

    let sender_email = configuration
        .email
        .sender()
        .expect("Invalid sender email address");
    let timeout = configuration.email.timeout();
    let email_client = EmailClient::new(
        configuration.email.base_url,
        sender_email,
        configuration.email.authorization_token,
        timeout,
    );

    let server = zero2prod::startup::run(listener, db_pool.clone(), email_client)
        .expect("Failed to run the app");
    #[allow(clippy::let_underscore_future)]
    let _ = tokio::spawn(server);

    let address = format!("http://127.0.0.1:{}", port);

    TestApp { address, db_pool }
}
