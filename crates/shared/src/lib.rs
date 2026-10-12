use std::{env, net::SocketAddr};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("DATABASE_URL must be set and nonempty")]
    DatabaseUrl,
    #[error("BIND_ADDR must be a valid socket address")]
    BindAddress,
    #[error("CORS_ALLOWED_ORIGINS must contain explicit http(s) origins without paths")]
    CorsOrigin,
}

pub struct Config {
    pub database_url: String,
    pub bind_addr: SocketAddr,
    pub cors_origins: Vec<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url = env::var("DATABASE_URL").map_err(|_| ConfigError::DatabaseUrl)?;
        if database_url.trim().is_empty() {
            return Err(ConfigError::DatabaseUrl);
        }
        let bind_addr = env::var("BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:3000".into())
            .parse()
            .map_err(|_| ConfigError::BindAddress)?;
        let cors_origins = parse_origins(&env::var("CORS_ALLOWED_ORIGINS").unwrap_or_default())?;
        Ok(Self {
            database_url,
            bind_addr,
            cors_origins,
        })
    }
}

pub fn parse_origins(value: &str) -> Result<Vec<String>, ConfigError> {
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|origin| {
            let url = url::Url::parse(origin).map_err(|_| ConfigError::CorsOrigin)?;
            if !matches!(url.scheme(), "http" | "https")
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.path() != "/"
                || url.query().is_some()
                || url.fragment().is_some()
            {
                return Err(ConfigError::CorsOrigin);
            }
            Ok(url.origin().ascii_serialization())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_origins() {
        assert!(parse_origins("").unwrap().is_empty());
        assert_eq!(
            parse_origins(" https://example.com/, http://localhost:5173 ").unwrap(),
            ["https://example.com", "http://localhost:5173"]
        );
        for value in [
            "*",
            "https://example.com/path",
            "https://user@example.com",
            "null",
            "https://example.com?q=1",
        ] {
            assert!(parse_origins(value).is_err(), "{value}");
        }
    }
}
