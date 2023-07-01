use reqwest::StatusCode;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::{spawn_app, ConfirmationLinks, TestApp};

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
    let response = test_app.post_newsletters(newsletter_request_body).await;

    assert_eq!(StatusCode::OK, response.status());
}

#[tokio::test]
async fn newsletters_are_sent_to_confirmed_subscribers() {
    let test_app = spawn_app().await;
    create_confirmed_subscriber(&test_app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as a plain text",
            "html": "<p>Newsletter body as HTML</p>",
        },
    });

    let response = test_app.post_newsletters(newsletter_request_body).await;

    assert_eq!(StatusCode::OK, response.status());
}

#[tokio::test]
async fn newsletters_returns_400_if_title_is_missing() {
    let test_app = spawn_app().await;
    let invalid_request = serde_json::json!({
        "content": {
            "text": "A plain text content.",
            "html": "<p>A fancy HTML content.</p>",
        },
    });
    let response = test_app.post_newsletters(invalid_request).await;

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}

#[tokio::test]
async fn newsletters_returns_400_if_content_is_missing() {
    let test_app = spawn_app().await;
    let invalid_request = serde_json::json!({
        "title": "My fancy newsletter",
    });
    let response = test_app.post_newsletters(invalid_request).await;

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}

async fn create_unconfirmed_subscriber(test_app: &TestApp) -> ConfirmationLinks {
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

    let email_request = test_app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    test_app.get_confirmation_links(&email_request)
}

async fn create_confirmed_subscriber(test_app: &TestApp) {
    let confirmation_links = create_unconfirmed_subscriber(test_app).await;
    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
