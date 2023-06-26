use crate::helpers::spawn_app;
use reqwest::StatusCode;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400_status() {
    let test_app = spawn_app().await;

    let response = reqwest::get(&format!("{}/subscriptions/confirm", test_app.address))
        .await
        .unwrap();

    assert_eq!(StatusCode::BAD_REQUEST, response.status());
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_200_status_when_called() {
    let test_app = spawn_app().await;
    let body = "email=harry.potter%40hogwarts.school&name=Harry%20Potter";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.into()).await;

    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_links(email_request);

    let confirmation_response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(StatusCode::OK, confirmation_response.status());
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber() {
    let test_app = spawn_app().await;
    let body = "name=James%20Brown&email=james%40brown.name";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.into()).await;

    let received_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_links(received_request);

    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let persisted_subscriber =
        sqlx::query!("SELECT `email`, `name`, `status` FROM `subscriptions`")
            .fetch_one(&test_app.db_pool)
            .await
            .expect("Failed to fetch the saved subscriber.");

    assert_eq!("james@brown.name", persisted_subscriber.email);
    assert_eq!("James Brown", persisted_subscriber.name);
    assert_eq!("confirmed", persisted_subscriber.status);
}
