use actix_web::{http::header::ContentType, HttpResponse, Responder};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

pub async fn login_form(flash_messages: IncomingFlashMessages) -> impl Responder {
    let mut message_html = String::new();

    for m in flash_messages.iter() {
        write!(message_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
<html>
    <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8" />
        <title>Login</title>
    </head>
    <body>
        {message_html}
        <form action="/login" method="post">
            <label>
                Username
                <input type="text" placeholder="Enter Username" name="username" />
            </label>
            <label>
                Password
                <input type="password" placeholder="Enter Password" name="password" />
            </label>
            <button type="submit">Login</button>
        </form>
    </body>
</html>"#,
        ))
}
