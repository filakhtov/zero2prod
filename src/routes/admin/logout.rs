use actix_web::{http::header::LOCATION, HttpResponse};
use actix_web_flash_messages::FlashMessage;

use crate::{
    session_state::TypedSession,
    utils::{internal_server_error, see_other},
};

pub async fn log_out(session: TypedSession) -> Result<HttpResponse, actix_web::Error> {
    if session
        .get_user_id()
        .map_err(internal_server_error)?
        .is_none()
    {
        return Ok(see_other("/login"));
    }

    session.purge();

    FlashMessage::info("You have successfully logged out").send();

    Ok(HttpResponse::SeeOther()
        .insert_header((LOCATION, "/login"))
        .finish())
}
