use crate::domain::SubscriberEmail;
use reqwest::Client;
use secrecy::{ExposeSecret, Secret};
use std::time::Duration;

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

pub struct EmailClient {
    base_url: String,
    http_client: Client,
    sender: SubscriberEmail,
    authorization_token: Secret<String>,
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
        timeout: Duration,
    ) -> Self {
        let http_client = Client::builder().timeout(timeout).build().unwrap();
        Self {
            base_url,
            http_client,
            sender,
            authorization_token,
        }
    }

    pub async fn send_email(
        &self,
        recipient: &SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let request_uri = format!("{}/email", self.base_url);
        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };
        self.http_client
            .post(&request_uri)
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use claims::{assert_err, assert_ok};
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use std::time::Duration;
    use wiremock::{
        matchers::{any, header, header_exists, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    struct SendEmailBodyMatcher {}

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &wiremock::Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);

            if let Ok(body) = result {
                return body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some();
            }

            false
        }
    }

    fn subject() -> String {
        fake::faker::lorem::en::Sentence(1..2).fake()
    }

    fn content() -> String {
        fake::faker::lorem::en::Paragraph(1..10).fake()
    }

    fn email() -> super::SubscriberEmail {
        let email: String = fake::faker::internet::en::SafeEmail().fake();

        super::SubscriberEmail::parse(&email).unwrap()
    }

    fn email_client(base_url: String) -> super::EmailClient {
        super::EmailClient::new(
            base_url,
            email(),
            Secret::new(Faker.fake()),
            Duration::from_millis(200),
        )
    }

    #[tokio::test]
    async fn send_email_sends_a_correct_request() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher {})
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _ = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_responds_with_200() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_ok!(result);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_responds_with_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_err!(result);
    }

    #[tokio::test]
    async fn send_email_aborts_connection_if_the_server_takes_too_long_to_respond() {
        let mock_server = MockServer::start().await;

        let email_client = email_client(mock_server.uri());

        let response = ResponseTemplate::new(200).set_delay(Duration::from_secs(180));
        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_err!(result);
    }
}
