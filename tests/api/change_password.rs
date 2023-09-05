use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn user_must_be_logged_in_see_the_change_password_form() {
    let test_app = spawn_app().await;

    let response = test_app.get_change_password().await;

    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn user_must_be_logged_in_to_change_the_password() {
    let test_app = spawn_app().await;

    let response = test_app
        .post_change_password(&serde_json::json!({
            "current_password": "old password",
            "new_password": "new password",
            "new_password_check": "new password",
        }))
        .await;

    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn new_password_fields_must_match() {
    let test_app = spawn_app().await;

    test_app
        .post_login(&serde_json::json!({
            "username": &test_app.test_user.username,
            "password": &test_app.test_user.password,
        }))
        .await;

    let response = test_app
        .post_change_password(&serde_json::json!({
            "current_password": &test_app.test_user.password,
            "new_password": "new pasword",
            "new_password_check": "new password check",
        }))
        .await;

    assert_is_redirect_to(&response, "/admin/password");

    let html_page = test_app.get_change_password_html().await;

    assert!(html_page.contains("<p><i>New passwords do not match</i></p>"));
}

#[tokio::test]
async fn current_password_must_be_valid() {
    let test_app = spawn_app().await;

    test_app
        .post_login(&serde_json::json!({
            "username": &test_app.test_user.username,
            "password": &test_app.test_user.password,
        }))
        .await;

    let response = test_app
        .post_change_password(&serde_json::json!({
            "current_password": "wrong password",
            "new_password": "new password",
            "new_password_check": "new password",
        }))
        .await;

    assert_is_redirect_to(&response, "/admin/password");

    let html_page = test_app.get_change_password_html().await;

    assert!(html_page.contains("<p><i>The current password is incorrect.</i></p>"));
}
