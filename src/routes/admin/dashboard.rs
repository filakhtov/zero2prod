use crate::authentication::UserId;
use actix_web::{
    error::ErrorInternalServerError, http::header::ContentType, web, Error, HttpResponse,
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
pub async fn get_username(user_id: Uuid, db_pool: &MySqlPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(r#"SELECT `username` FROM `users` WHERE `id` = ?"#, user_id)
        .fetch_one(db_pool)
        .await
        .context("Failed to perform a query to retrieve a username.")?;

    Ok(row.username)
}
pub async fn admin_dashboard(
    user_id: web::ReqData<UserId>,
    db_pool: web::Data<MySqlPool>,
) -> Result<HttpResponse, Error> {
    let user_id = user_id.into_inner();
    let username = get_username(*user_id, &db_pool)
        .await
        .map_err(internal_server_error)?;

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
        <p>Available actions:</p>
        <ol>
            <li>
                <form name="logoutForm" action="/admin/logout" method="post">
                    <input type="submit" value="logout" />
                </form>
            </li>
        </ol><ol>
            <li><a href="/admin/password">Change password</a></li>
        </ol>
    </body>
</html>"#
        )))
}
