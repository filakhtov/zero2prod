use config::{Config, ConfigError, File, FileFormat};

#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application_port: u16,
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

impl DatabaseSettings {
    pub fn connection_dsn(&self) -> String {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database_name
        )
    }
}

pub fn get_configuration(path: &str) -> Result<Settings, ConfigError> {
    let settings = Config::builder()
        .add_source(File::new(path, FileFormat::Yaml))
        .build()?;

    settings.try_deserialize()
}
