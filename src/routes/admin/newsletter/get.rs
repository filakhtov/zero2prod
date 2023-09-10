use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

pub async fn publish_newsletter_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let mut message_html = String::new();

    for message in flash_messages.iter() {
        write!(message_html, "<p><i>{}</i></p>", message.content()).unwrap();
    }

    let idempotency_key = uuid::Uuid::new_v4();

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
        <html lang="en">
            <head>
                <meta http-equiv="content-type" content="text/html; charset=utf-8" />
                <title>Newsletter publishing</title>
            </head>
            <body>
                <ul>{message_html}</ul>
                <form action="/admin/newsletter" method="post">
                    <input hidden="hidden" type="text" name="idempotency_key" value="{idempotency_key}" />
                    <label for="title">
                        Newsletter title:
                        <input type="text" name="title" placeholder="Title">
                    </label>
                    <br />
                    <label for="text_content">
                        Content for plain text clients:<br />
                        <textarea name="text_content"></textarea>
                    </label>
                    <br />
                    <label for="html_content">
                        Content for HTML-enabled clients:<br />
                        <textarea name="html_content"></textarea>
                    </label>
                    <br />
                    <button type="submit">Send</button>
                </form>
            </body>
        </html>"#
        ))
}
