use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

use crate::{
    session_state::TypedSession,
    utils::{internal_server_error, see_other},
};

pub async fn change_password_form(
    session: TypedSession,
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    if session
        .get_user_id()
        .map_err(internal_server_error)?
        .is_none()
    {
        return Ok(see_other("/login"));
    }

    let mut msg_html = String::new();
    for message in flash_messages.iter() {
        writeln!(msg_html, "<p><i>{}</i></p>", message.content()).unwrap();
    }

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
    <html lang="en">
        <head>
            <meta http-equiv="content-type" content="text/html; charset=utf-8" />
            <title>Change Password</title>
        </head>
        <body>
            {msg_html}
            <form action="/admin/password" method="post">
                <label>Current Password:
                    <input type="password" placeholder="Enter current password" name="current_password" />
                </label>
                <br>
                <label>New Password:
                    <input type="password" placeholder="Enter the new password" name="new_password" />
                </label>
                <br>
                <label>Confirm New Password:
                    <input type="password" placeholder="Repeat new password" name="new_password_check" />
                </label>
                <br>
                <button type="submit">Change Password</button>
            </form>
        </body>
    </html>"#
        )))
}
