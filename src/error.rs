use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum Application {
    #[error("Command `{command}` failed wiht: `{error_text}`")]
    CommandFailed {
        command: &'static str,
        error_text: String,
    },
    #[error("Incorrect `{category}` value `{value}` found.")]
    InvalidValue {
        category: &'static str,
        value: String,
    },
    #[error("The value `{value}` is not a currently supported {category}.")]
    UnsupportedValue {
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
