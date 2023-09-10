use crate::helpers::{assert_is_redirect_to, spawn_app, ConfirmationLinks, TestApp};
use wiremock::{self, matchers};

#[tokio::test]
async fn newsletter_publishing_require_user_to_be_authenticated() {
    let test_app = spawn_app().await;

    let response = test_app.get_publish_newsletter().await;
    assert_is_redirect_to(&response, "/login");

    let response = test_app
        .post_publish_newsletter(&serde_json::json!({
            "title": "Amazing neswletter",
            "text_content": "This is a newsletter body",
            "html_content": "<p>This is a newsletter body</p>",
            "idempotency_key": uuid::Uuid::new_v4().to_string(),
        }))
        .await;
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn newsletter_is_not_sent_to_unconfirmed_subscribers() {
    let test_app = spawn_app().await;
    create_unconfirmed_subscriber(&test_app).await;

    test_app.test_user.login(&test_app).await;

    wiremock::Mock::given(matchers::any())
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(0)
        .mount(&test_app.email_server)
        .await;

    let response = test_app
        .post_publish_newsletter(&serde_json::json!({
            "title": "Newsletter title",
            "text_content": "Newsletter body as plain text",
            "html_content": "<p>Newsletter body as HTML</p>",
            "idempotency_key": uuid::Uuid::new_v4().to_string(),
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/newsletter");

    let html_content = test_app.get_publish_newsletter_html().await;
    assert!(html_content
        .contains("<p><i>The newsletter issues has been published successfully</i></p>"));
}

#[tokio::test]
async fn newsletter_is_sent_to_confirmed_subscribers() {
    let test_app = spawn_app().await;
    create_confirmed_subscriber(&test_app).await;

    test_app.test_user.login(&test_app).await;

    wiremock::Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let response = test_app
        .post_publish_newsletter(&serde_json::json!({
            "title": "Newsletter title",
            "text_content": "Newsletter body as a plain text",
            "html_content": "<p>Newsletter body as HTML</p>",
            "idempotency_key": uuid::Uuid::new_v4().to_string(),
        }))
        .await;

    assert_is_redirect_to(&response, "/admin/newsletter");

    let html_content = test_app.get_publish_newsletter_html().await;
    assert!(html_content
        .contains("<p><i>The newsletter issues has been published successfully</i></p>"));
}

#[tokio::test]
async fn newsletter_not_sent_if_content_is_missing_or_empty() {
    let test_app = spawn_app().await;

    test_app.test_user.login(&test_app).await;

    let response = test_app
        .post_publish_newsletter(&serde_json::json!({
            "title": "Newsletter title",
            "html_content": "<p>Newsletter body as HTML</p>",
            "text_content": "",
            "idempotency_key": uuid::Uuid::new_v4().to_string(),
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/newsletter");

    let html_content = test_app.get_publish_newsletter_html().await;
    assert!(html_content
        .contains("<p><i>Failed to publish the newsletter: missing text content</i></p>"));

    let response = test_app
        .post_publish_newsletter(&serde_json::json!({
            "title": "Newsletter title",
            "text_content": "Newsletter body as plain text",
            "html_content": "",
            "idempotency_key": uuid::Uuid::new_v4().to_string(),
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/newsletter");

    let html_content = test_app.get_publish_newsletter_html().await;
    assert!(html_content
        .contains("<p><i>Failed to publish the newsletter: missing HTML content</i></p>"));
}

#[tokio::test]
async fn newsletter_not_sent_if_title_is_missing() {
    let test_app = spawn_app().await;

    test_app.test_user.login(&test_app).await;

    let response = test_app
        .post_publish_newsletter(&serde_json::json!({
            "title": "",
            "html_content": "<p>Newsletter body as HTML</p>",
            "text_content": "Newsletter body as plain text",
            "idempotency_key": uuid::Uuid::new_v4().to_string(),
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/newsletter");

    let html_content = test_app.get_publish_newsletter_html().await;
    assert!(html_content
        .contains("<p><i>Failed to publish the newsletter: missing newsletter title</i></p>"));
}

#[tokio::test]
async fn newsletter_publishing_is_idempotent() {
    let test_app = spawn_app().await;
    create_confirmed_subscriber(&test_app).await;
    test_app.test_user.login(&test_app).await;

    wiremock::Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let request_body = serde_json::json!({
        "title": "My next newsletter",
        "html_content": "<p>This is yet another HTML email</p>",
        "text_content": "This is yet another text email",
        "idempotency_key": uuid::Uuid::new_v4().to_string(),
    });
    let response = test_app.post_publish_newsletter(&request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletter");

    let html_content = test_app.get_publish_newsletter_html().await;
    assert!(html_content
        .contains("<p><i>The newsletter issues has been published successfully</i></p>"));

    let response = test_app.post_publish_newsletter(&request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletter");

    let html_content = test_app.get_publish_newsletter_html().await;
    assert!(html_content
        .contains("<p><i>The newsletter issues has been published successfully</i></p>"));
}

async fn create_unconfirmed_subscriber(test_app: &TestApp) -> ConfirmationLinks {
    let body = "name=Joseph%20Stutgart&email=jstutgart@example.com";

    let _mock_guard = wiremock::Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(200))
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
