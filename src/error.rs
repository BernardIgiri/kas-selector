use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum Application {
    #[error("Initialization failed for `{category}` due to `{cause}`")]
    FailedToInitialize {
        category: &'static str,
        cause: String,
    },
    #[error("Incorrect `{category}` value `{value}` found.")]
    BadInitData {
        category: &'static str,
        value: String,
    },
    #[error("Failed to save script `{script_path}` for activity `{activity}` and event `{event}`.")]
    SaveDataError {
        activity: String,
        event: &'static str,
        script_path: String,
    },
}

pub use Application::*;
