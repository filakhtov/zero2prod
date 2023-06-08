use std::net::TcpListener;

use reqwest::StatusCode;

#[tokio::test]
async fn health_check_responds_with_204_and_no_content() {
    let port = spawn_app().await;

    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://127.0.0.1:{}/health_check", port))
        .send()
        .await
        .expect("Failed to send a request to our app");

    assert!(response.status().is_success());
    assert_eq!(StatusCode::NO_CONTENT, response.status());
    assert_eq!(Some(0), response.content_length());
}

async fn spawn_app() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to the random port");
    let port = listener.local_addr().unwrap().port();
    let server = z2p::run(listener).expect("Failed to bind the address");
    let _ = tokio::spawn(server);

    port
}
