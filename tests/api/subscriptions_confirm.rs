use crate::helpers::spawn_app;
use reqwest::StatusCode;

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400_status() {
    let test_app = spawn_app().await;

    let response = reqwest::get(&format!("{}/subscriptions/confirm", test_app.address))
        .await
        .unwrap();

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}
