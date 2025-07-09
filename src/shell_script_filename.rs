use derive_more::{AsRef, Debug, Display};
use std::{path::Path, str::FromStr};

use crate::error::Application;

#[derive(Debug, Display, AsRef, Clone, PartialEq, Eq, Hash)]
pub struct ShellScriptFilename(String);

impl ShellScriptFilename {
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl FromStr for ShellScriptFilename {
    type Err = Application;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() || s == "." || s == ".." || s.contains('\0') || s.len() > 255 {
            return Err(Self::error(s));
        }

        let path = Path::new(s);

        if path.components().count() != 1 {
            return Err(Self::error(s));
        }

        if !matches!(path.extension().and_then(|ext| ext.to_str()), Some("sh")) {
            return Err(Self::error(s));
        }

        if path.file_name().is_none() {
            return Err(Self::error(s));
        }

        Ok(Self(s.to_owned()))
    }
}

impl ShellScriptFilename {
    fn error(s: &str) -> Application {
        Application::BadInitData {
            category: "ShellScriptFilename",
            value: s.to_owned(),
        }
    }
}
