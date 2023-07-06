use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn an_error_message_is_set_on_failure() {
    let test_app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": "random-user",
        "password": "wrong-password",
    });

    let response = test_app.post_login(&login_body).await;

    assert_is_redirect_to(&response, "/login");

    let flash_cookie = response.cookies().find(|c| c.name() == "_flash").unwrap();
    assert_eq!("Authentication failed", flash_cookie.value());

    let html_page = test_app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>Authentication failed</i></p>"#));

    let html_page = test_app.get_login_html().await;
    assert!(!html_page.contains(r#"<p><i>Authentication failed</i></p>"#));
}
