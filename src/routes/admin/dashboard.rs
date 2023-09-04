use actix_session::Session;
use actix_web::{
    error::ErrorInternalServerError, http::header::ContentType, web, Error, HttpResponse, Responder,
};
use anyhow::Context;
use sqlx::MySqlPool;
use uuid::Uuid;

fn internal_server_error<T>(e: T) -> Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    ErrorInternalServerError(e)
}

#[tracing::instrument(name = "Get username", skip(db_pool))]
async fn get_username(user_id: Uuid, db_pool: &MySqlPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"SELECT `username` FROM `users` WHERE `id` = ?"#,
        user_id.to_string()
    )
    .fetch_one(db_pool)
    .await
    .context("Failed to perform a query to retrieve a username.")?;

    Ok(row.username)
}

pub async fn admin_dashboard(
    session: Session,
    db_pool: web::Data<MySqlPool>,
) -> Result<impl Responder, Error> {
    let username = if let Some(user_id) = session
        .get::<Uuid>("user_id")
        .map_err(internal_server_error)?
    {
        get_username(user_id, &db_pool)
            .await
            .map_err(internal_server_error)?
    } else {
        todo!();
    };

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
<html lang="en">
    <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8" />
        <title>Admin dashboard</title>
    </head>
    <body>
        <p>Welcome {username}!</p>
    </body>
</html>"#
        )))
}
