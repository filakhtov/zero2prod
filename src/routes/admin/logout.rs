use crate::session_state::TypedSession;
use actix_web::{http::header::LOCATION, HttpResponse};
use actix_web_flash_messages::FlashMessage;

pub async fn log_out(session: TypedSession) -> Result<HttpResponse, actix_web::Error> {
    session.purge();

    FlashMessage::info("You have successfully logged out").send();

    Ok(HttpResponse::SeeOther()
        .insert_header((LOCATION, "/login"))
        .finish())
}
