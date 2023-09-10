use super::IdempotencyKey;
use actix_web::{body::to_bytes, http::StatusCode, HttpResponse};
use sqlx::{Decode, Encode, MySql, MySqlPool, Transaction};
use uuid::Uuid;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct HeaderCollection(Vec<(String, Vec<u8>)>);

impl sqlx::Type<MySql> for HeaderCollection {
    fn type_info() -> <MySql as sqlx::Database>::TypeInfo {
        <[u8] as sqlx::Type<MySql>>::type_info()
    }
}

impl<'r> Decode<'r, MySql> for HeaderCollection {
    fn decode(
        value: <MySql as sqlx::database::HasValueRef<'r>>::ValueRef,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let decoded = <&'r [u8] as Decode<MySql>>::decode(value)?;
        Ok(serde_json::from_slice(decoded)?)
    }
}

impl<'q> Encode<'q, MySql> for HeaderCollection {
    fn encode_by_ref(
        &self,
        buf: &mut <MySql as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        let enc = serde_json::to_vec(self).unwrap();
        <Vec<u8> as Encode<'q, MySql>>::encode_by_ref(&enc, buf)
    }
}

pub async fn get_saved_response(
    db_pool: &MySqlPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Result<Option<HttpResponse>, anyhow::Error> {
    let saved_response = sqlx::query!(
        r#"SELECT `response_status_code` as "response_status_code!",
                  `response_headers` as "response_headers!: HeaderCollection",
                  `response_body` as "response_body!"
             FROM `idempotency`
            WHERE `user_id` = ? AND idempotency_key = ?
            LIMIT 1"#,
        user_id.to_string(),
        idempotency_key.as_ref(),
    )
    .fetch_optional(db_pool)
    .await?;

    if let Some(r) = saved_response {
        let status_code = StatusCode::from_u16(r.response_status_code.try_into()?)?;
        let mut response = HttpResponse::build(status_code);
        for header in r.response_headers.0 {
            response.append_header(header);
        }

        return Ok(Some(response.body(r.response_body)));
    }

    Ok(None)
}

pub async fn save_response(
    mut transaction: Transaction<'static, MySql>,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
    http_response: HttpResponse,
) -> Result<HttpResponse, anyhow::Error> {
    let (response, body) = http_response.into_parts();
    let body = to_bytes(body).await.map_err(|e| anyhow::anyhow!("{}", e))?;
    let status_code = response.status().as_u16() as i16;
    let headers = {
        let headers = response.headers();
        let mut h = Vec::with_capacity(headers.len());
        for (name, value) in headers.iter() {
            h.push((name.as_str().to_owned(), value.as_bytes().to_owned()));
        }

        HeaderCollection(h)
    };

    sqlx::query!(
        r#"UPDATE `idempotency` SET `response_status_code`=?, `response_headers`=?, `response_body`=?
            WHERE `user_id`=? AND `idempotency_key`=?"#,
        status_code,
        headers,
        body.as_ref(),
        user_id.to_string(),
        idempotency_key.as_ref(),
    ).execute(&mut transaction).await?;

    transaction.commit().await?;

    let http_response = response.set_body(body).map_into_boxed_body();
    Ok(http_response)
}

pub enum NextAction {
    StartProcessing(Transaction<'static, MySql>),
    ReturnSavedResponse(HttpResponse),
}

pub async fn try_processing(
    db_pool: &MySqlPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Result<NextAction, anyhow::Error> {
    let mut transaction = db_pool.begin().await?;
    let inserted_rows_count = sqlx::query!(
        r#"INSERT IGNORE INTO `idempotency` (
            `user_id`, `idempotency_key`, `created_at`
        ) VALUES (?, ?, CURRENT_TIMESTAMP())"#,
        user_id.to_string(),
        idempotency_key.as_ref(),
    )
    .execute(&mut transaction)
    .await?
    .rows_affected();

    if inserted_rows_count > 0 {
        Ok(NextAction::StartProcessing(transaction))
    } else {
        let saved_response = get_saved_response(db_pool, idempotency_key, user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Unable to find a saved response"))?;
        Ok(NextAction::ReturnSavedResponse(saved_response))
    }
}
