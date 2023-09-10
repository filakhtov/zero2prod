use crate::helpers::{assert_is_redirect_to, spawn_app, ConfirmationLinks, TestApp};
use fake::{
    faker::{internet::en::SafeEmail, name::en::Name},
    Fake,
};
use std::time::Duration;
use wiremock::{self, matchers};

fn when_sending_an_email() -> wiremock::MockBuilder {
    wiremock::Mock::given(matchers::path("/email")).and(matchers::method("POST"))
}

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
    assert!(html_content.contains(
        "<p><i>The newsletter issues has been accepted \
        and emails will be sent out shortly</i></p>",
    ));

    test_app.dispatch_all_pending_emails().await;
}

#[tokio::test]
async fn newsletter_is_sent_to_confirmed_subscribers() {
    let test_app = spawn_app().await;
    create_confirmed_subscriber(&test_app).await;

    test_app.test_user.login(&test_app).await;

    when_sending_an_email()
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
    assert!(html_content.contains(
        "<p><i>The newsletter issues has been accepted \
        and emails will be sent out shortly</i></p>",
    ));

    test_app.dispatch_all_pending_emails().await;
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

    when_sending_an_email()
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
    assert!(html_content.contains(
        "<p><i>The newsletter issues has been accepted \
        and emails will be sent out shortly</i></p>",
    ));

    test_app.dispatch_all_pending_emails().await;

    let response = test_app.post_publish_newsletter(&request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletter");

    let html_content = test_app.get_publish_newsletter_html().await;
    assert!(html_content.contains(
        "<p><i>The newsletter issues has been accepted \
        and emails will be sent out shortly</i></p>",
    ));

    test_app.dispatch_all_pending_emails().await;
}

#[tokio::test]
async fn concurrent_form_submissions_are_handled_idempotently() {
    let test_app = spawn_app().await;
    create_confirmed_subscriber(&test_app).await;
    test_app.test_user.login(&test_app).await;

    when_sending_an_email()
        .respond_with(wiremock::ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let request_body = serde_json::json!({
        "title": "My concurrent newsletter submission",
        "html_content": "<p>Things are going to happen <i>in parallel</i> with this HTML</p>",
        "text_content": "Things are going to happen IN PARALLEL with this text",
        "idempotency_key": uuid::Uuid::new_v4().to_string(),
    });

    let response1 = test_app.post_publish_newsletter(&request_body);
    let response2 = test_app.post_publish_newsletter(&request_body);
    let (response1, response2) = tokio::join!(response1, response2);

    assert_eq!(response1.status(), response2.status());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap(),
    );

    test_app.dispatch_all_pending_emails().await;
}

async fn create_unconfirmed_subscriber(test_app: &TestApp) -> ConfirmationLinks {
    let name: String = Name().fake();
    let email: String = SafeEmail().fake();
    let body = serde_urlencoded::to_string(serde_json::json!({
        "name": name,
        "email": email
    }))
    .unwrap();

    let _mock_guard = wiremock::Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&test_app.email_server)
        .await;

    test_app
        .post_subscriptions(body)
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
