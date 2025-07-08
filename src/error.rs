use thiserror::Error;

#[derive(Debug, Error)]
pub enum Application {
    #[error("Initialization failed for {category} due to {cause}")]
    FailedToInitialize {
        category: &'static str,
        cause: String,
    },
    #[error("Failed to initialized! Data for {category} was incorrect value {value}")]
    BadInitData {
        category: &'static str,
        value: String,
    },
    #[error("Failed to save script {script_path} for activity {activity} and event {event}.")]
    SaveDataError {
        activity: String,
        event: &'static str,
        script_path: String,
    },
}

pub use Application::*;
