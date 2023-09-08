use fake::Fake;

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
    test_app.test_user.login(&test_app).await;

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

    test_app.test_user.login(&test_app).await;

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

#[tokio::test]
async fn new_password_must_be_at_least_12_characters_long() {
    let test_app = spawn_app().await;

    test_app.test_user.login(&test_app).await;

    let response = test_app
        .post_change_password(&serde_json::json!({
            "current_password": test_app.test_user.password,
            "new_password": "short",
            "new_password_check": "short",
        }))
        .await;

    assert_is_redirect_to(&response, "/admin/password");

    let html_content = test_app.get_change_password_html().await;

    assert!(
        html_content.contains("<p><i>Password must be between 12 and 128 characters long</i></p>")
    );
}

#[tokio::test]
async fn new_password_must_be_at_most_128_characters_long() {
    let test_app = spawn_app().await;

    test_app.test_user.login(&test_app).await;

    let too_long_password = (130..140).fake::<String>();

    let response = test_app
        .post_change_password(&serde_json::json!({
            "current_password": test_app.test_user.password,
            "new_password": too_long_password,
            "new_password_check": too_long_password,
        }))
        .await;

    assert_is_redirect_to(&response, "/admin/password");

    let html_content = test_app.get_change_password_html().await;

    assert!(
        html_content.contains("<p><i>Password must be between 12 and 128 characters long</i></p>")
    );
}

#[tokio::test]
async fn password_changing_works() {
    let test_app = spawn_app().await;

    test_app.test_user.login(&test_app).await;

    let response = test_app
        .post_change_password(&serde_json::json!({
            "current_password": test_app.test_user.password,
            "new_password": "my new password",
            "new_password_check": "my new password",
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/password");

    let html_content = test_app.get_change_password_html().await;
    assert!(html_content.contains("<p><i>Your password was successfully changed</i></p>"));

    let response = test_app.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    let html_content = test_app.get_login_html().await;
    assert!(html_content.contains("<p><i>You have successfully logged out</i></p>"));

    let response = test_app
        .post_login(&serde_json::json!({
            "username": test_app.test_user.username,
            "password": "my new password",
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/dashboard");
}
