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

    let html_page = test_app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>Authentication failed</i></p>"#));

    let html_page = test_app.get_login_html().await;
    assert!(!html_page.contains(r#"<p><i>Authentication failed</i></p>"#));
}

#[tokio::test]
async fn redirect_to_admin_dashboard_after_successful_login() {
    let test_app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": &test_app.test_user.username,
        "password": &test_app.test_user.password,
    });

    let response = test_app.post_login(&login_body).await;

    assert_is_redirect_to(&response, "/admin/dashboard");

    let html_page = test_app.get_admin_dashboard().await;

    assert!(html_page.contains(&format!("Welcome {}", test_app.test_user.username)));
}
