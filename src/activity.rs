use std::{
    collections::HashMap,
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::Command,
    sync::LazyLock,
};

use derive_getters::Getters;
use regex::Regex;
use strum::{Display, EnumIter, IntoEnumIterator, IntoStaticStr};

use crate::{error, locale, shell_script_filename::ShellScriptFilename};

type EventMap = HashMap<ActivityEvent, PathBuf>;
type ScriptMap = HashMap<String, EventMap>;

#[allow(clippy::expect_used)]
static ACTIVITY_DATA_RX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*\[\w+\]\s+(?P<id>[a-f0-9\-]+)\s+(?P<name>.+?)\s+\([^\n]+\)\s*$")
        .expect("ValidRx")
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, Display, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
pub enum ActivityEvent {
    Activated,
    Deactivated,
    Started,
    Stopped,
}

impl ActivityEvent {
    pub const fn as_key(&self) -> locale::Key {
        use locale::Key as K;
        match self {
            Self::Activated => K::EventActivated,
            Self::Deactivated => K::EventDeactivated,
            Self::Started => K::EventStarted,
            Self::Stopped => K::EventStopped,
        }
    }
}

#[derive(Debug, Getters, Clone)]
pub struct Activity {
    name: String,
    id: String,
    #[getter(skip)]
    event_scripts: EventMap,
}

impl Activity {
    pub fn get_script(&self, event: &ActivityEvent) -> Option<&PathBuf> {
        self.event_scripts.get(event)
    }
    pub fn set_script(&mut self, event: ActivityEvent, script: PathBuf) {
        self.event_scripts.insert(event, script);
    }
    pub fn delete_script(&mut self, event: ActivityEvent) {
        self.event_scripts.remove(&event);
    }
    pub fn from_env(
        root_folder: &Path,
        script_filename: &ShellScriptFilename,
    ) -> Result<Vec<Self>, error::Application> {
        let output = Command::new("kactivities-cli")
            .arg("--list-activities")
            .output()
            .map_err(|e| error::Application::CommandFailed {
                command: "kactivities-cli",
                error_text: e.to_string(),
            })?;

        if !output.status.success() {
            let error_text = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(error::Application::CommandFailed {
                command: "kactivities-cli",
                error_text,
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let scripts = Self::load_scripts(root_folder, script_filename)?;
        Self::from_activity_data(&stdout, scripts)
    }
    fn load_scripts(
        root: &Path,
        script_filename: &ShellScriptFilename,
    ) -> Result<ScriptMap, error::Application> {
        let mut scripts = ScriptMap::new();
        for entry in fs::read_dir(root).map_err(|e| error::InvalidValue {
            category: "reading root script directory",
            value: e.to_string(),
        })? {
            let activity_dir = entry
                .map_err(|e| error::InvalidValue {
                    category: "reading entry in root script directory",
                    value: e.to_string(),
                })?
                .path();

            if !activity_dir.is_dir() {
                continue;
            }
            let activity_id = activity_dir
                .file_name()
                .ok_or_else(|| error::InvalidValue {
                    category: "reading activity folder name",
                    value: activity_dir.to_string_lossy().to_string(),
                })?
                .to_string_lossy()
                .to_string();

            let mut event_map = EventMap::new();
            for event_entry in fs::read_dir(&activity_dir).map_err(|e| error::InvalidValue {
                category: "reading event folder list",
                value: e.to_string(),
            })? {
                let event_path = event_entry
                    .map_err(|e| error::InvalidValue {
                        category: "reading event folder entry",
                        value: e.to_string(),
                    })?
                    .path();
                if !event_path.is_dir() {
                    continue;
                }
                let event_name = event_path
                    .file_name()
                    .ok_or_else(|| error::InvalidValue {
                        category: "reading event folder name",
                        value: event_path.to_string_lossy().to_string(),
                    })?
                    .to_string_lossy()
                    .to_string();
                if let Some(event) = match event_name.as_str() {
                    "activated" => Some(ActivityEvent::Activated),
                    "deactivated" => Some(ActivityEvent::Deactivated),
                    "started" => Some(ActivityEvent::Started),
                    "stopped" => Some(ActivityEvent::Stopped),
                    _ => None,
                } {
                    let script_path = event_path.join(script_filename.as_str());
                    if script_path.exists() {
                        event_map.insert(event, script_path);
                    }
                }
            }
            if !event_map.is_empty() {
                scripts.insert(activity_id, event_map);
            }
        }
        Ok(scripts)
    }
    pub fn from_activity_data(
        data: &str,
        scripts: ScriptMap,
    ) -> Result<Vec<Self>, error::Application> {
        data.lines()
            .filter_map(|line| ACTIVITY_DATA_RX.captures(line))
            .map(|cap| {
                let id = cap
                    .name("id")
                    .ok_or_else(|| error::InvalidValue {
                        category: "Activity Data id",
                        value: data.to_string(),
                    })?
                    .as_str()
                    .to_string();
                let name = cap
                    .name("name")
                    .ok_or_else(|| error::InvalidValue {
                        category: "Activity Data name",
                        value: data.to_string(),
                    })?
                    .as_str()
                    .to_string();
                let event_scripts = scripts.get(&id).cloned().unwrap_or_default();
                Ok(Self {
                    name,
                    id,
                    event_scripts,
                })
            })
            .collect()
    }
    pub fn save_activities(
        root: &Path,
        script_filename: &ShellScriptFilename,
        activities: &[Self],
    ) -> Result<(), error::Application> {
        for activity in activities {
            for event in ActivityEvent::iter() {
                let script = activity.get_script(&event);
                let dest_dir = root.join(&activity.id).join(event.to_string());
                let dest_path = dest_dir.join(script_filename.as_str());
                if dest_path.exists() {
                    fs::remove_file(&dest_path).map_err(|_| error::SaveDataError {
                        activity: activity.name().clone(),
                        event: event.into(),
                        script_path: dest_path.to_string_lossy().into(),
                    })?;
                }
                if let Some(script_path) = script {
                    fs::create_dir_all(&dest_dir).map_err(|_| error::SaveDataError {
                        activity: activity.name().clone(),
                        event: event.into(),
                        script_path: dest_path.to_string_lossy().into(),
                    })?;
                    symlink(script_path, &dest_path).map_err(|_| error::SaveDataError {
                        activity: activity.name().clone(),
                        event: event.into(),
                        script_path: dest_path.to_string_lossy().into(),
                    })?;
                }
            }
        }

        Ok(())
    }
}

// Allowed in tests
#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use std::fs::symlink_metadata;

    use super::*;
    use asserting::prelude::*;
    use tempfile::tempdir;

    #[test]
    fn debug_regex_capture() {
        let line = "[RUNNING] abc-12d-a Activity A (icon-a)";
        let cap = ACTIVITY_DATA_RX.captures(line);
        assert!(cap.is_some(), "Regex failed to match: {line}");
    }
    #[test]
    fn from_activity_data_contains_exactly() {
        let sample_data = r#"
            [RUNNING] abc-12d-a Activity A (icon-a)
            [STOPPED] abc-12d-b activity B (icon-b)
            [CURRENT] abc-12d-d Long Named Activity (icon-d)
            [RUNNING] abc-12d-e Filing Taxes & Accounting (icon-e)
        "#
        .trim();

        let activities = Activity::from_activity_data(sample_data, ScriptMap::new()).unwrap();

        let actual: Vec<_> = activities
            .iter()
            .map(|a| (a.name().clone(), a.id().clone()))
            .collect();

        assert_that!(actual).contains_exactly([
            ("Activity A".to_string(), "abc-12d-a".to_string()),
            ("activity B".to_string(), "abc-12d-b".to_string()),
            ("Long Named Activity".to_string(), "abc-12d-d".to_string()),
            (
                "Filing Taxes & Accounting".to_string(),
                "abc-12d-e".to_string(),
            ),
        ]);
    }
    #[test]
    fn from_activity_data_populates_event_scripts() {
        let sample_data = r#"
            [RUNNING] abc-12d-a Activity A (icon-a)
            [RUNNING] abc-12d-b Activity B (icon-b)
        "#
        .trim();

        let mut map = ScriptMap::new();

        let mut events_a = EventMap::new();
        events_a.insert(
            ActivityEvent::Activated,
            PathBuf::from("/scripts/a/activated/kas-script.sh"),
        );
        events_a.insert(
            ActivityEvent::Started,
            PathBuf::from("/scripts/a/started/kas-script.sh"),
        );

        let mut events_b = EventMap::new();
        events_b.insert(
            ActivityEvent::Deactivated,
            PathBuf::from("/scripts/b/deactivated/kas-script.sh"),
        );

        map.insert("abc-12d-a".into(), events_a.clone());
        map.insert("abc-12d-b".into(), events_b.clone());

        let activities = Activity::from_activity_data(sample_data, map).unwrap();

        assert_that!(activities.len()).is_equal_to(2);

        let a = activities.iter().find(|a| a.id() == "abc-12d-a").unwrap();
        assert_that!(a.event_scripts.clone()).is_equal_to(events_a);

        let b = activities.iter().find(|a| a.id() == "abc-12d-b").unwrap();
        assert_that!(b.event_scripts.clone()).is_equal_to(events_b);
    }
    #[test]
    fn load_scripts_reads_symlink_structure() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let activity_id = "abc-12d-x";
        let event_name = "activated";

        let activity_dir = root.join(activity_id).join(event_name);
        fs::create_dir_all(&activity_dir).unwrap();

        let actual_script = root.join("actual-script.sh");
        fs::write(&actual_script, "#!/bin/sh\necho Hello").unwrap();

        let symlink_path = activity_dir.join("kas-script.sh");
        symlink(&actual_script, &symlink_path).unwrap();

        let result = Activity::load_scripts(root, &"kas-script.sh".parse().unwrap()).unwrap();

        assert_that!(result.len()).is_equal_to(1);
        let event_map = result.get(activity_id).unwrap();

        assert_that!(event_map.len()).is_equal_to(1);
        assert_that!(event_map.get(&ActivityEvent::Activated).unwrap()).is_equal_to(&symlink_path);
    }
    #[test]
    fn save_activities_writes_symlink_structure() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();

        let source_script = root.join("hello.sh");
        fs::write(&source_script, "#!/bin/sh\necho hello").unwrap();

        let mut events = EventMap::new();
        events.insert(ActivityEvent::Started, source_script.clone());

        let activity = Activity {
            name: "TestActivity".into(),
            id: "a-1".into(),
            event_scripts: events,
        };

        Activity::save_activities(root, &"kas-script.sh".parse().unwrap(), &[activity]).unwrap();

        let link_path = root.join("a-1/started/kas-script.sh");
        let meta = symlink_metadata(&link_path).unwrap();
        assert!(meta.file_type().is_symlink());

        let target = fs::read_link(link_path).unwrap();
        assert_eq!(target, source_script);
    }
    #[test]
    fn save_activities_removes_unlinked_scripts() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();

        let source_script = root.join("hello.sh");
        fs::write(&source_script, "#!/bin/sh\necho hello").unwrap();

        // Manually create a stale symlink that should be removed
        let link_dir = root.join("a-1/started");
        fs::create_dir_all(&link_dir).unwrap();
        let link_path = link_dir.join("kas-script.sh");
        symlink(&source_script, &link_path).unwrap();
        assert!(link_path.exists(), "Expected initial symlink to be present");

        // Now save an activity without a script for that event
        let activity = Activity {
            name: "TestActivity".into(),
            id: "a-1".into(),
            event_scripts: EventMap::new(),
        };

        Activity::save_activities(root, &"kas-script.sh".parse().unwrap(), &[activity]).unwrap();

        assert!(
            !link_path.exists(),
            "Expected symlink to be removed when event is unset"
        );
    }
}
