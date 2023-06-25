use crate::helpers::spawn_app;
use reqwest::StatusCode;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn subscribe_responds_with_200_for_valid_form_data() {
    let test_app = spawn_app().await;

    let body = "name=John%20Doe&email=john.doe%40example.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    let response = test_app.post_subscriptions(body.into()).await;

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

    let body_without_email = "name=Alice%20Smith";
    let response = test_app.post_subscriptions(body_without_email.into()).await;

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}

#[tokio::test]
async fn subscribe_responds_with_400_when_name_is_missing() {
    let test_app = spawn_app().await;

    let body_without_name = "email=alice.smith%40example.com";
    let response = test_app.post_subscriptions(body_without_name.into()).await;

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}

#[tokio::test]
async fn subscribe_responds_with_400_when_email_and_name_are_missing() {
    let test_app = spawn_app().await;

    let empty_body = "";
    let response = test_app.post_subscriptions(empty_body.into()).await;

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}

#[tokio::test]
async fn subscribe_responds_with_400_when_email_is_present_but_empty() {
    let test_app = spawn_app().await;

    let body_with_empty_email = "email=&name=Anthony";
    let response = test_app
        .post_subscriptions(body_with_empty_email.into())
        .await;

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}

#[tokio::test]
async fn subscribe_responds_with_400_when_name_is_invalid() {
    let test_app = spawn_app().await;

    let body_with_empty_name = "email=anthony.muir@example.com&name=";
    let response = test_app
        .post_subscriptions(body_with_empty_name.into())
        .await;

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}

#[tokio::test]
async fn subscriber_responds_with_400_when_email_has_invalid_format() {
    let test_app = spawn_app().await;

    let body_with_invalid_email = "email=nonsense&name=Bill";
    let response = test_app
        .post_subscriptions(body_with_invalid_email.into())
        .await;

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    let test_app = spawn_app().await;

    let body = "name=Jonathan&email=jonathan.white%40example.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.into()).await;
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let test_app = spawn_app().await;
    let body = "name=John%20Wick&email=john.wick@example.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.into()).await;

    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
    let get_link = |content: &str| {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(content)
            .filter(|link| *link.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(links.len(), 1);
        links[0].as_str().to_owned()
    };
    let html_link = get_link(body["HtmlBody"].as_str().unwrap());
    let text_link = get_link(body["TextBody"].as_str().unwrap());

    assert_eq!(html_link, text_link);
}
