use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    errors::error_chain_fmt,
    session_state::TypedSession,
};
use actix_web::{error::InternalError, http::header::LOCATION, web, HttpResponse, Responder};
use actix_web_flash_messages::FlashMessage;
use secrecy::Secret;
use sqlx::MySqlPool;

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrond")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

fn login_redirect(e: LoginError) -> InternalError<LoginError> {
    FlashMessage::error(e.to_string()).send();

    let response = HttpResponse::SeeOther()
        .insert_header((LOCATION, "/login"))
        .finish();

    InternalError::from_response(e, response)
}

#[tracing::instrument(
    name = "Login",
    skip(form, db_pool, session),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty),
)]
pub async fn login(
    db_pool: web::Data<MySqlPool>,
    form: web::Form<FormData>,
    session: TypedSession,
) -> Result<impl Responder, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };

    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    match validate_credentials(credentials, &db_pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

            session.renew();
            session
                .insert_user_id(user_id)
                .map_err(|e| login_redirect(LoginError::UnexpectedError(e.into())))?;

            Ok(HttpResponse::SeeOther()
                .insert_header((LOCATION, "/admin/dashboard"))
                .finish())
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                _ => LoginError::UnexpectedError(e.into()),
            };

            Err(login_redirect(e))
        }
    }
}
