use config::{Config, ConfigError, File, FileFormat};
use secrecy::{ExposeSecret, Secret};

#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
}

#[derive(serde::Deserialize)]
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
}

impl DatabaseSettings {
    pub fn database_dsn(&self) -> Secret<String> {
        Secret::new(format!(
            "{}/{}",
            self.connection_dsn().expose_secret(),
            self.database_name
        ))
    }

    pub fn connection_dsn(&self) -> Secret<String> {
        Secret::new(format!(
            "mysql://{}:{}@{}:{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
        ))
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
