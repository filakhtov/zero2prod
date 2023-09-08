use crate::{
    authentication::UserId,
    domain::SubscriberEmail,
    email_client::EmailClient,
    utils::{internal_server_error, see_other},
};
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::MySqlPool;

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    text_content: String,
    html_content: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Publish a newsletter", skip(body, pool, email_client))]
pub async fn publish_newsletter(
    pool: web::Data<MySqlPool>,
    body: web::Form<BodyData>,
    email_client: web::Data<EmailClient>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    if body.text_content.is_empty() {
        FlashMessage::error("Failed to publish the newsletter: missing text content").send();

        return Ok(see_other("/admin/newsletter"));
    }

    if body.html_content.is_empty() {
        FlashMessage::error("Failed to publish the newsletter: missing HTML content").send();

        return Ok(see_other("/admin/newsletter"));
    }

    if body.title.is_empty() {
        FlashMessage::error("Failed to publish the newsletter: missing newsletter title").send();

        return Ok(see_other("/admin/newsletter"));
    }

    let mut count = 0;
    let subscribers = get_confirmed_subscribers(&pool)
        .await
        .map_err(internal_server_error)?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.html_content,
                        &body.text_content,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send a newseletter issue to {}", subscriber.email)
                    })
                    .map_err(internal_server_error)?;

                count += 1;
            }
            Err(error) => {
                tracing::warn!(error.cause_chain = ?error, "Skipping a confirmed subscriber. \
            Their stored email address is invalid.");
            }
        }
    }

    FlashMessage::info(format!(
        "Newsletter successfully sent to {count} subscriber(s)"
    ))
    .send();

    Ok(see_other("/admin/newsletter"))
}

#[tracing::instrument(name = "Get a list of confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &MySqlPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let rows = sqlx::query!(
        r#"
            SELECT `email`
            FROM `subscriptions`
            WHERE `status`="confirmed"
        "#,
    )
    .fetch_all(pool)
    .await?;

    let confirmed_subscribers = rows
        .into_iter()
        .map(|row| match SubscriberEmail::parse(&row.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(anyhow::anyhow!(error)),
        })
        .collect();

    Ok(confirmed_subscribers)
}
