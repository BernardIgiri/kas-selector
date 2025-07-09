use std::path::PathBuf;

use derive_getters::Getters;
use derive_new::new;

use crate::shell_script_filename::ShellScriptFilename;

#[derive(Debug, Getters, new, Clone)]
pub struct Config {
    root_path: PathBuf,
    script_filename: ShellScriptFilename,
}
