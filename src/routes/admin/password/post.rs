use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};
use sqlx::MySqlPool;

use crate::{
    session_state::TypedSession,
    utils::{internal_server_error, see_other},
};

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

pub async fn change_password(
    session: TypedSession,
    form_data: web::Form<FormData>,
    db_pool: web::Data<MySqlPool>,
) -> Result<HttpResponse, actix_web::Error> {
    if session
        .get_user_id()
        .map_err(internal_server_error)?
        .is_none()
    {
        return Ok(see_other("/login"));
    }

    if form_data.new_password.expose_secret() != form_data.new_password_check.expose_secret() {
        FlashMessage::error("New passwords do not match").send();

        return Ok(see_other("/admin/password"));
    }

    todo!()
}
