use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    errors::error_chain_fmt,
    startup::HmacSecret,
};
use actix_web::{error::InternalError, http::header::LOCATION, web, HttpResponse, Responder};
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, Secret};
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

#[tracing::instrument(
    name = "Login",
    skip(form, db_pool, secret),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty),
)]
pub async fn login(
    db_pool: web::Data<MySqlPool>,
    form: web::Form<FormData>,
    secret: web::Data<HmacSecret>,
) -> Result<impl Responder, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };

    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    match validate_credentials(credentials, &db_pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            Ok(HttpResponse::SeeOther()
                .insert_header((LOCATION, "/"))
                .finish())
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                _ => LoginError::UnexpectedError(e.into()),
            };

            let query_string = format!("error={}", urlencoding::Encoded::new(e.to_string()));
            let hmac_tag = {
                let mut mac =
                    Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes())
                        .unwrap();
                mac.update(query_string.as_bytes());
                mac.finalize().into_bytes()
            };

            let response = HttpResponse::SeeOther()
                .insert_header((
                    LOCATION,
                    format!("/login?{}&tag={:x}", query_string, hmac_tag),
                ))
                .finish();

            Err(InternalError::from_response(e, response))
        }
    }
}
