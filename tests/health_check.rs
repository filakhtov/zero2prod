use reqwest::{Client, StatusCode};
use sqlx::{Executor, MySqlPool};
use std::net::TcpListener;
use uuid::Uuid;
use zero2prod::configuration::{get_configuration, DatabaseSettings};

pub struct TestApp {
    pub address: String,
    pub db_pool: MySqlPool,
}

async fn spawn_app() -> TestApp {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to the random port");
    let port = listener.local_addr().unwrap().port();

    let mut configuration =
        get_configuration("test.yaml").expect("Failed to read test configuration");
    configuration.database.database_name = format!(
        "{}_{}",
        configuration.database.database_name,
        Uuid::new_v4(),
    );
    let db_pool = configure_database(&configuration.database).await;

    let server = zero2prod::startup::run(listener, db_pool.clone()).expect("Failed to run the app");
    let _ = tokio::spawn(server);

    let address = format!("http://127.0.0.1:{}", port);

    TestApp { address, db_pool }
}

async fn configure_database(db_settings: &DatabaseSettings) -> MySqlPool {
    let db_pool = MySqlPool::connect(&db_settings.connection_dsn())
        .await
        .expect("Failed to connect to the database");
    db_pool
        .execute(format!(r#"CREATE DATABASE `{}`;"#, db_settings.database_name).as_str())
        .await
        .expect("Failed to create database");

    let db_pool = MySqlPool::connect(&db_settings.database_dsn())
        .await
        .expect("Failed to connect to the database");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to run migrations");

    db_pool
}

#[tokio::test]
async fn health_check_responds_with_204_and_no_content() {
    let test_app = spawn_app().await;

    let client = Client::new();

    let response = client
        .get(format!("{}/health_check", test_app.address))
        .send()
        .await
        .expect("Failed to send a request to the app");

    assert!(response.status().is_success());
    assert_eq!(StatusCode::NO_CONTENT, response.status());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_responds_with_200_for_valid_form_data() {
    let test_app = spawn_app().await;
    let client = Client::new();

    let body = "name=John%20Doe&email=john.doe%40example.com";
    let response = client
        .post(&format!("{}/subscriptions", &test_app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to send a request to the app");

    assert_eq!(StatusCode::OK, response.status());

    let saved = sqlx::query!("SELECT `email`, `name` FROM `subscriptions`")
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch persisted subscription");

    assert_eq!(saved.email, "john.doe@example.com");
    assert_eq!(saved.name, "John Doe");
}

#[tokio::test]
async fn subscribe_responds_with_400_when_email_is_missing() {
    let test_app = spawn_app().await;
    let client = Client::new();

    let body_without_email = "name=Alice%20Smith";
    let response = client
        .post(format!("{}/subscriptions", &test_app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body_without_email)
        .send()
        .await
        .expect("Failed to send a request to the app");

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}

#[tokio::test]
async fn subscribe_responds_with_400_when_name_is_missing() {
    let test_app = spawn_app().await;
    let client = Client::new();

    let body_without_name = "email=alice.smith%40example.com";
    let response = client
        .post(format!("{}/subscriptions", &test_app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body_without_name)
        .send()
        .await
        .expect("Failed to send a request to the app");

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}

#[tokio::test]
async fn subscribe_responds_with_400_when_email_and_name_are_missing() {
    let test_app = spawn_app().await;
    let client = Client::new();

    let empty_body = "";
    let response = client
        .post(format!("{}/subscriptions", &test_app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(empty_body)
        .send()
        .await
        .expect("Failed to send a request to the app");

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}
