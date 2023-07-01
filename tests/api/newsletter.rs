use reqwest::StatusCode;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::{spawn_app, TestApp};

#[tokio::test]
async fn newsletters_are_not_sent_to_unconfirmed_subscribers() {
    let test_app = spawn_app().await;
    create_unconfirmed_subscriber(&test_app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&test_app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        },
    });
    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &test_app.address))
        .json(&newsletter_request_body)
        .send()
        .await
        .expect("Failed to send a request to the app");

    assert_eq!(StatusCode::OK, response.status());
}

async fn create_unconfirmed_subscriber(test_app: &TestApp) {
    let body = "name=Joseph%20Stutgart&email=jstutgart@example.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&test_app.email_server)
        .await;

    test_app
        .post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();
}
