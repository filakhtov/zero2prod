use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sqlx::{MySql, MySqlPool, Transaction};
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(&form.name)?;
        let email = SubscriberEmail::parse(&form.email)?;
        Ok(NewSubscriber { name, email })
    }
}

#[tracing::instrument(name = "Persisting subscriber", skip(new_subscriber, db_transaction))]
async fn persist_subscriber(
    db_transaction: &mut Transaction<'_, MySql>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO `subscriptions` (`id`, `email`, `name`, `subscribed_at`, `status`)
        VALUES (?, ?, ?, ?, "pending_confirmation")
        "#,
        subscriber_id.to_string(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
    )
    .execute(db_transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute the query: {:?}", e);

        e
    })?;

    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url, subscription_token)
)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token,
    );
    let plain_body = format!(
        "Welcome to our newsletter\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    let html_body = format!(
        "Welcome to our newsletter!<br />\
                Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email(new_subscriber.email, "Welcome", &html_body, &plain_body)
        .await?;

    Ok(())
}

fn generate_subscription_token() -> String {
    let rng = thread_rng();
    rng.sample_iter(Alphanumeric)
        .map(char::from)
        .take(25)
        .collect()
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, db_transaction)
)]
async fn persist_token(
    db_transaction: &mut Transaction<'_, MySql>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO `subscription_tokens` (`subscription_token`, `subscriber_id`) VALUES (?, ?)"#,
        subscription_token,
        subscriber_id.to_string(),
    ).execute(db_transaction).await.map_err(|e| {tracing::error!("Failed to execute query: {:?}", e); e})?;
    Ok(())
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, db_pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    db_pool: web::Data<MySqlPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> impl Responder {
    let new_subscriber = match form.0.try_into() {
        Ok(sub) => sub,
        _ => return HttpResponse::BadRequest().finish(),
    };
    let mut db_transaction = match db_pool.begin().await {
        Ok(t) => t,
        _ => return HttpResponse::InternalServerError().finish(),
    };

    let subscriber_id = match persist_subscriber(&mut db_transaction, &new_subscriber).await {
        Ok(id) => id,
        _ => return HttpResponse::InternalServerError().finish(),
    };

    let subscription_token = &generate_subscription_token();
    if persist_token(&mut db_transaction, subscriber_id, subscription_token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    if db_transaction.commit().await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    if send_confirmation_email(
        email_client.as_ref(),
        new_subscriber,
        &base_url.0,
        subscription_token,
    )
    .await
    .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}
