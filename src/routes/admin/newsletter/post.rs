use crate::{
    authentication::UserId,
    domain::SubscriberEmail,
    email_client::EmailClient,
    idempotency::{save_response, try_processing, IdempotencyKey, NextAction},
    utils::{bad_request, internal_server_error, see_other},
};
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::MySqlPool;

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    text_content: String,
    html_content: String,
    idempotency_key: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Publish a newsletter", skip(form, db_pool, email_client))]
pub async fn publish_newsletter(
    db_pool: web::Data<MySqlPool>,
    form: web::Form<FormData>,
    email_client: web::Data<EmailClient>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let FormData {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = form.0;
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(bad_request)?;

    if text_content.is_empty() {
        FlashMessage::error("Failed to publish the newsletter: missing text content").send();

        return Ok(see_other("/admin/newsletter"));
    }

    if html_content.is_empty() {
        FlashMessage::error("Failed to publish the newsletter: missing HTML content").send();

        return Ok(see_other("/admin/newsletter"));
    }

    if title.is_empty() {
        FlashMessage::error("Failed to publish the newsletter: missing newsletter title").send();

        return Ok(see_other("/admin/newsletter"));
    }

    let transaction = match try_processing(&db_pool, &idempotency_key, *user_id)
        .await
        .map_err(internal_server_error)?
    {
        NextAction::StartProcessing(transaction) => transaction,
        NextAction::ReturnSavedResponse(saved_response) => {
            success_message().send();

            return Ok(saved_response);
        }
    };

    let subscribers = get_confirmed_subscribers(&db_pool)
        .await
        .map_err(internal_server_error)?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(&subscriber.email, &title, &html_content, &text_content)
                    .await
                    .with_context(|| {
                        format!("Failed to send a newseletter issue to {}", subscriber.email)
                    })
                    .map_err(internal_server_error)?;
            }
            Err(error) => {
                tracing::warn!(error.cause_chain = ?error, "Skipping a confirmed subscriber. \
                Their stored email address is invalid.");
            }
        }
    }

    success_message().send();

    let response = see_other("/admin/newsletter");
    let response = save_response(transaction, &idempotency_key, *user_id, response)
        .await
        .map_err(internal_server_error)?;

    Ok(response)
}

fn success_message() -> FlashMessage {
    FlashMessage::info("The newsletter issues has been published successfully")
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
