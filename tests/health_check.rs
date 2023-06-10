use reqwest::{Client, StatusCode};
use std::net::TcpListener;

fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to the random port");
    let port = listener.local_addr().unwrap().port();
    let server = zero2prod::startup::run(listener).expect("Failed to run the app");
    let _ = tokio::spawn(server);

    format!("http://127.0.0.1:{}", port)
}

#[tokio::test]
async fn health_check_responds_with_204_and_no_content() {
    let address = spawn_app();

    let client = Client::new();

    let response = client
        .get(format!("{}/health_check", address))
        .send()
        .await
        .expect("Failed to send a request to the app");

    assert!(response.status().is_success());
    assert_eq!(StatusCode::NO_CONTENT, response.status());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_responds_with_200_for_valid_form_data() {
    let address = spawn_app();
    let client = Client::new();

    let body = "name=john%20doe&email=john.doe%40example.com";
    let response = client
        .post(&format!("{}/subscriptions", &address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to send a request to the app");

    assert_eq!(StatusCode::OK, response.status())
}

#[tokio::test]
async fn subscribe_responds_with_400_when_email_is_missing() {
    let address = spawn_app();
    let client = Client::new();

    let body_without_email = "name=Alice%20Smith";
    let response = client
        .post(format!("{}/subscriptions", &address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body_without_email)
        .send()
        .await
        .expect("Failed to send a request to the app");

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}

#[tokio::test]
async fn subscribe_responds_with_400_when_name_is_missing() {
    let address = spawn_app();
    let client = Client::new();

    let body_without_name = "email=alice.smith%40example.com";
    let response = client
        .post(format!("{}/subscriptions", &address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body_without_name)
        .send()
        .await
        .expect("Failed to send a request to the app");

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}

#[tokio::test]
async fn subscribe_responds_with_400_when_email_and_name_are_missing() {
    let address = spawn_app();
    let client = Client::new();

    let empty_body = "";
    let response = client
        .post(format!("{}/subscriptions", &address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(empty_body)
        .send()
        .await
        .expect("Failed to send a request to the app");

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}
