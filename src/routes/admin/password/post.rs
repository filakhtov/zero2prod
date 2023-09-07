use actix_web::{error::InternalError, web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    routes::admin::dashboard::get_username,
    session_state::TypedSession,
    utils::{internal_server_error, see_other},
};

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

fn reject_anonymous_users(session: TypedSession) -> Result<Uuid, actix_web::Error> {
    let user_id = session.get_user_id().map_err(internal_server_error)?;
    match user_id {
        Some(id) => Ok(id),
        _ => {
            let response = see_other("/login");
            let e = anyhow::anyhow!("The user has not logged in");
            Err(InternalError::from_response(e, response).into())
        }
    }
}

pub async fn change_password(
    session: TypedSession,
    form_data: web::Form<FormData>,
    db_pool: web::Data<MySqlPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = reject_anonymous_users(session)?;

    let new_password = form_data.0.new_password.expose_secret();
    if new_password != form_data.0.new_password_check.expose_secret() {
        FlashMessage::error("New passwords do not match").send();

        return Ok(see_other("/admin/password"));
    }

    if new_password.len() < 12 || new_password.len() > 128 {
        FlashMessage::error("Password must be between 12 and 128 characters long").send();

        return Ok(see_other("/admin/password"));
    }

    let username = get_username(user_id, &db_pool)
        .await
        .map_err(internal_server_error)?;

    let credentials = Credentials {
        username,
        password: form_data.0.current_password,
    };

    if let Err(e) = validate_credentials(credentials, &db_pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect.").send();
                Ok(see_other("/admin/password"))
            }
            _ => Err(internal_server_error(e)),
        };
    }

    crate::authentication::change_password(user_id, form_data.0.new_password, &db_pool)
        .await
        .map_err(internal_server_error)?;

    FlashMessage::info("Your password was successfully changed").send();

    Ok(see_other("/admin/password"))
}
