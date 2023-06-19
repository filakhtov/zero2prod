use crate::helpers::spawn_app;
use reqwest::{Client, StatusCode};

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
