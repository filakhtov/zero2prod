use reqwest::Url;
use sqlx::{Executor, MySqlPool};
use tokio::sync::OnceCell;
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup::{get_connection_pool, Application},
    telemetry::{get_subscriber, init_subscriber},
};

pub struct ConfirmationLinks {
    pub html: Url,
    pub text: Url,
}

pub struct TestApp {
    pub address: String,
    pub db_pool: MySqlPool,
    pub email_server: MockServer,
    pub port: u16,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to send a request")
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        let (username, password) = self.test_user().await;

        reqwest::Client::new()
            .post(&format!("{}/newsletters", self.address))
            .basic_auth(username, Some(password))
            .json(&body)
            .send()
            .await
            .expect("Failed to send a request to `/subscriptions` endpoint")
    }

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_link = |text: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(text)
                .filter(|link| *link.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(1, links.len());
            let mut confirmation_link = Url::parse(links[0].as_str()).unwrap();
            assert_eq!("127.0.0.1", confirmation_link.host_str().unwrap());
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let text = get_link(body["TextBody"].as_str().unwrap());

        ConfirmationLinks { html, text }
    }

    pub async fn test_user(&self) -> (String, String) {
        let row = sqlx::query!("SELECT `username`, `password` FROM `users` LIMIT 1")
            .fetch_one(&self.db_pool)
            .await
            .expect("Failed to fetch test user credentials.");

        (row.username, row.password)
    }
}

async fn cleanup_database(db_settings: &DatabaseSettings) {
    let db_connection = MySqlPool::connect_with(db_settings.without_db())
        .await
        .expect("Failed to connect to the database");
    let rows = match sqlx::query!(
        r#"
        SELECT DISTINCT `table_schema`
        FROM `information_schema`.`tables`
        WHERE `table_schema` LIKE 'newsletter_%'
        "#
    )
    .fetch_all(&db_connection)
    .await
    {
        Ok(rows) => rows,
        _ => return,
    };

    for row in rows {
        let _ = db_connection
            .execute(format!(r#"DROP DATABASE `{}`"#, row.table_schema).as_str())
            .await;
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
    let email_server = MockServer::start().await;

    let configuration = {
        let mut conf = get_configuration("test.yaml").expect("Failed to read test configuration");
        conf.database.database_name = format!(
            "{}_{}",
            conf.database.database_name,
            Uuid::new_v4().as_simple(),
        );
        conf.email.base_url = email_server.uri();
        conf
    };
    let db_configuration = configuration.database.clone();

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

    create_database(&db_configuration).await;
    migrate_database(&db_configuration).await;

    let application = Application::build(configuration)
        .await
        .expect("Failed to build the application.");
    let port = application.port();
    let address = format!("http://127.0.0.1:{}", port);

    #[allow(clippy::let_underscore_future)]
    let _ = tokio::spawn(application.run_until_stopped());

    let db_pool = get_connection_pool(&db_configuration);

    add_test_user(&db_pool).await;

    TestApp {
        address,
        db_pool,
        email_server,
        port,
    }
}

async fn add_test_user(db_pool: &MySqlPool) {
    sqlx::query!(
        "
            INSERT INTO `users` (`id`, `username`, `password`)
            VALUES (?, ?, ?)
        ",
        Uuid::new_v4().to_string(),
        Uuid::new_v4().to_string(),
        Uuid::new_v4().to_string(),
    )
    .execute(db_pool)
    .await
    .expect("Failed to create test users");
}
