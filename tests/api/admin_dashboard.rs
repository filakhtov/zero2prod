use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn user_must_be_logged_in_to_access_the_admin_dashboard() {
    let test_app = spawn_app().await;

    let response = test_app.get_admin_dashboard().await;

    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn logout_clears_session_state() {
    let test_app = spawn_app().await;

    let response = test_app
        .post_login(&serde_json::json!({
            "username": test_app.test_user.username,
            "password": test_app.test_user.password,
        }))
        .await;

    assert_is_redirect_to(&response, "/admin/dashboard");

    let html_content = test_app.get_admin_dashboard_html().await;
    assert!(html_content.contains(&format!("Welcome {}", test_app.test_user.username)));

    let response = test_app.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    let html_content = test_app.get_login_html().await;
    assert!(html_content.contains(r#"<p><i>You have successfully logged out</i></p>"#));

    let response = test_app.get_admin_dashboard().await;
    assert_is_redirect_to(&response, "/login");
}
