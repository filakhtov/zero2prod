use std::time::Duration;

use config::{Config, ConfigError, File, FileFormat};
use secrecy::{ExposeSecret, Secret};
use sqlx::{mysql::MySqlConnectOptions, ConnectOptions};
use tracing::log::LevelFilter;

use crate::domain::SubscriberEmail;

#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    pub email: EmailClientSettings,
}

#[derive(Clone, serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

#[derive(serde::Deserialize)]
pub struct ApplicationSettings {
    pub host: String,
    pub port: u16,
    pub base_url: String,
    pub hmac_secret: Secret<String>,
}

#[derive(serde::Deserialize)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    pub authorization_token: Secret<String>,
    pub timeout_milliseconds: u64,
}

impl EmailClientSettings {
    pub fn sender(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(&self.sender_email)
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_milliseconds)
    }
}

impl DatabaseSettings {
    pub fn without_db(&self) -> MySqlConnectOptions {
        let mut options = MySqlConnectOptions::new()
            .host(&self.host)
            .port(self.port)
            .username(&self.username)
            .password(self.password.expose_secret());
        options.log_statements(LevelFilter::Trace);
        options
    }

    pub fn with_db(&self) -> MySqlConnectOptions {
        self.without_db().database(&self.database_name)
    }
}

pub fn get_configuration(path: &str) -> Result<Settings, ConfigError> {
    let settings = Config::builder()
        .add_source(File::new(path, FileFormat::Yaml))
        .add_source(
            config::Environment::with_prefix("ZERO2PROD")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;

    settings.try_deserialize()
}
