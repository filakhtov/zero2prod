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
