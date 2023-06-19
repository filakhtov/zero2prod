use crate::helpers::spawn_app;
use reqwest::{Client, StatusCode};

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

#[tokio::test]
async fn subscribe_responds_with_400_when_email_is_present_but_empty() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let body = "email=&name=Anthony";
    let response = client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to send a request to the app");

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}

#[tokio::test]
async fn subscribe_responds_with_400_when_name_is_invalid() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let body = "email=anthony.muir@example.com&name=";
    let response = client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to send a request to the app");

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}

#[tokio::test]
async fn subscriber_responds_with_400_when_email_has_invalid_format() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let body = "email=nonsense&name=Bill";
    let response = client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to send a request to the app");

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}
