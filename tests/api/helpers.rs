use argon2::{password_hash::SaltString, Algorithm, Argon2, Params, PasswordHasher, Version};
use reqwest::{StatusCode, Url};
use sqlx::{Executor, MySqlPool};
use tokio::sync::OnceCell;
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    email_client::EmailClient,
    issue_delivery_worker::{try_execute_task, ExecutionOutcome},
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
    pub test_user: TestUser,
    pub api_client: reqwest::Client,
    pub email_client: EmailClient,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to send a request")
    }

    pub async fn get_login_html(&self) -> String {
        self.api_client
            .get(format!("{}/login", self.address))
            .send()
            .await
            .expect("Failed to send a request to the app")
            .text()
            .await
            .unwrap()
    }

    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(format!("{}/login", self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to send a request to the app")
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

    pub async fn get_admin_dashboard(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/dashboard", self.address))
            .send()
            .await
            .expect("Failed to send a request to the app")
    }

    pub async fn get_admin_dashboard_html(&self) -> String {
        self.get_admin_dashboard().await.text().await.unwrap()
    }

    pub async fn get_change_password(&self) -> reqwest::Response {
        self.api_client
            .get(format!("{}/admin/password", self.address))
            .send()
            .await
            .expect("Failed to send a request to the app")
    }

    pub async fn get_change_password_html(&self) -> String {
        self.get_change_password().await.text().await.unwrap()
    }

    pub async fn post_change_password<Body>(&self, form_data: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(format!("{}/admin/password", self.address))
            .form(form_data)
            .send()
            .await
            .expect("Failed to send a request to the app")
    }

    pub async fn post_logout(&self) -> reqwest::Response {
        self.api_client
            .post(format!("{}/admin/logout", self.address))
            .send()
            .await
            .expect("Failed to send a request to the app")
    }

    pub async fn post_publish_newsletter<Body>(&self, form_data: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(format!("{}/admin/newsletter", self.address))
            .form(form_data)
            .send()
            .await
            .expect("Failed to send a request to the app")
    }

    pub async fn get_publish_newsletter(&self) -> reqwest::Response {
        self.api_client
            .get(format!("{}/admin/newsletter", self.address))
            .send()
            .await
            .expect("Failed to send a request to the app")
    }

    pub async fn get_publish_newsletter_html(&self) -> String {
        self.get_publish_newsletter().await.text().await.unwrap()
    }

    pub async fn dispatch_all_pending_emails(&self) {
        loop {
            if let ExecutionOutcome::EmptyQueue =
                try_execute_task(&self.db_pool, &self.email_client)
                    .await
                    .unwrap()
            {
                break;
            }
        }
    }
}

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    async fn persist(&self, pool: &MySqlPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();
        sqlx::query!(
            "INSERT INTO `users` (`id`, `username`, `password_hash`) VALUES (?, ?, ?)",
            self.user_id,
            self.username,
            password_hash,
        )
        .execute(pool)
        .await
        .expect("Failed to persist the test user");
    }

    pub async fn login(&self, app: &TestApp) {
        let response = app
            .post_login(&serde_json::json!({
                "username": self.username,
                "password": self.password,
            }))
            .await;

        assert_is_redirect_to(&response, "/admin/dashboard");
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

pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(StatusCode::SEE_OTHER, response.status());
    assert_eq!(response.headers().get("Location").unwrap(), location);
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

    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build the application.");
    let port = application.port();
    let address = format!("http://127.0.0.1:{}", port);

    #[allow(clippy::let_underscore_future)]
    let _ = tokio::spawn(application.run_until_stopped());

    let db_pool = get_connection_pool(&db_configuration);
    let test_user = TestUser::generate();

    test_user.persist(&db_pool).await;

    let api_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();

    TestApp {
        address,
        db_pool,
        email_server,
        port,
        test_user,
        api_client,
        email_client: configuration.email.client(),
    }
}
