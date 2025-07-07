use thiserror::Error;

#[derive(Debug, Error)]
pub enum Application {
    #[error("Invalid/missing {config} configuration for {identifier}")]
    InvalidConfiguration {
        config: &'static str,
        identifier: String,
    },
}

pub use Application::*;
