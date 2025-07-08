use std::path::PathBuf;

use derive_getters::Getters;
use derive_new::new;

#[derive(Debug, Getters, new, Clone)]
pub struct Config {
    root_path: PathBuf,
    script_filename: String,
}
