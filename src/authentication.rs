use anyhow::Context;
use argon2::{
    password_hash::SaltString, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier,
    Version,
};
use secrecy::{ExposeSecret, Secret};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::telemetry::spawn_blocking_with_tracing;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "Load stored credentials", skip(username, db_pool))]
async fn load_stored_credentials(
    username: &str,
    db_pool: &MySqlPool,
) -> Result<(Option<Uuid>, Secret<String>), anyhow::Error> {
    let result = sqlx::query!(
        r#"
            SELECT `id`, `password_hash`
              FROM `users`
             WHERE `username`=?
        "#,
        username,
    )
    .fetch_optional(db_pool)
    .await
    .context("Load stored credentials query failed.")?;

    match result {
        Some(row) => {
            let uuid = Uuid::parse_str(&row.id)
                .context("Failed to parse user UUID loaded from the database.")?;
            let password = Secret::new(row.password_hash);
            Ok((Some(uuid), password))
        }
        _ => Ok((
            None,
            Secret::new(
                "$argon2id$v=19$m=15000,t=2,p=1$\
                gZiV/M1gPc22ElAH/Jh1Hw$\
                CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
                    .to_string(),
            ),
        )),
    }
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse the PHC format string hash.")
        .map_err(AuthError::UnexpectedError)?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, db_pool))]
pub async fn validate_credentials(
    credentials: Credentials,
    db_pool: &MySqlPool,
) -> Result<Uuid, AuthError> {
    let (user_id, expected_password_hash) = load_stored_credentials(&credentials.username, db_pool)
        .await
        .map_err(AuthError::UnexpectedError)?;

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn a blocking task for password hashing")
    .map_err(AuthError::InvalidCredentials)??;

    user_id.ok_or_else(|| AuthError::InvalidCredentials(anyhow::anyhow!("Unknown username.")))
}

#[tracing::instrument(name = "Change password", skip(password, db_pool))]
pub async fn change_password(
    user_id: Uuid,
    password: Secret<String>,
    db_pool: &MySqlPool,
) -> Result<(), anyhow::Error> {
    let password_hash = spawn_blocking_with_tracing(move || compute_password_hash(password))
        .await?
        .context("Failed to hash password")?;

    sqlx::query!(
        r#"
        UPDATE `users`
           SET `password_hash` = ?
         WHERE `id` = ?
        "#,
        password_hash.expose_secret(),
        user_id.to_string(),
    )
    .execute(db_pool)
    .await
    .context("Failed to execute query to change the password.")?;

    Ok(())
}

fn compute_password_hash(password: Secret<String>) -> Result<Secret<String>, anyhow::Error> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::new(
        argon2::Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(password.expose_secret().as_bytes(), &salt)?
    .to_string();

    Ok(Secret::new(password_hash))
}
