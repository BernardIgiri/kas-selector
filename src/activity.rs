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
use strum::{Display, EnumIter, IntoStaticStr};

use crate::{error, locale};

type EventMap = HashMap<ActivityEvent, PathBuf>;
type ScriptMap = HashMap<String, EventMap>;

#[allow(clippy::expect_used)]
static ACTIVITY_DATA_RX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^[^"]*"(?<id>[^"]+)", "(?<name>[^"]+)""#).expect("ValidRx"));

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumIter, Display, IntoStaticStr)]
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
    event_scripts: EventMap,
}

impl Activity {
    pub fn set_script(&mut self, event: ActivityEvent, script: PathBuf) {
        self.event_scripts.insert(event, script);
    }
    pub fn delete_script(&mut self, event: ActivityEvent) {
        self.event_scripts.remove(&event);
    }
    pub fn from_env(
        root_folder: &Path,
        script_filename: &str,
    ) -> Result<Vec<Self>, error::Application> {
        let output = Command::new("qdbus")
            .args([
                "--literal",
                "org.kde.ActivityManager",
                "/ActivityManager/Activities",
                "ListActivitiesWithInformation",
            ])
            .output()
            .map_err(|e| error::Application::FailedToInitialize {
                category: "QBus data",
                cause: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(error::Application::FailedToInitialize {
                category: "qdbus ListActivitiesWithInformation",
                cause: stderr.trim().to_string(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let scripts = Self::load_scripts(root_folder, script_filename)?;
        Self::from_activity_data(&stdout, scripts)
    }
    fn load_scripts(root: &Path, script_filename: &str) -> Result<ScriptMap, error::Application> {
        let mut scripts = ScriptMap::new();
        for entry in fs::read_dir(root).map_err(|e| error::BadInitData {
            category: "reading root script directory",
            value: e.to_string(),
        })? {
            let activity_dir = entry
                .map_err(|e| error::BadInitData {
                    category: "reading entry in root script directory",
                    value: e.to_string(),
                })?
                .path();

            if !activity_dir.is_dir() {
                continue;
            }
            let activity_id = activity_dir
                .file_name()
                .ok_or_else(|| error::BadInitData {
                    category: "reading activity folder name",
                    value: activity_dir.to_string_lossy().to_string(),
                })?
                .to_string_lossy()
                .to_string();

            let mut event_map = EventMap::new();
            for event_entry in fs::read_dir(&activity_dir).map_err(|e| error::BadInitData {
                category: "reading event folder list",
                value: e.to_string(),
            })? {
                let event_path = event_entry
                    .map_err(|e| error::BadInitData {
                        category: "reading event folder entry",
                        value: e.to_string(),
                    })?
                    .path();
                if !event_path.is_dir() {
                    continue;
                }
                let event_name = event_path
                    .file_name()
                    .ok_or_else(|| error::BadInitData {
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
                    let script_path = event_path.join(script_filename);
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
        let cleaned = data
            .trim_start_matches(|c| c != '{')
            .trim_start_matches('{')
            .trim_end_matches("}]");

        let segments = cleaned.split("], [");

        let mut activities = Vec::new();

        for segment in segments {
            let line = segment.trim_matches(|c| c == '[' || c == ']');
            if let Some(cap) = ACTIVITY_DATA_RX.captures(line) {
                let id = cap
                    .name("id")
                    .ok_or_else(|| error::FailedToInitialize {
                        category: "Activity Data id",
                        cause: "missing".into(),
                    })?
                    .as_str()
                    .to_string();
                let name = cap
                    .name("name")
                    .ok_or_else(|| error::FailedToInitialize {
                        category: "Activity Data name",
                        cause: "missing".into(),
                    })?
                    .as_str()
                    .to_string();
                let event_scripts = scripts.get(&id).cloned().unwrap_or_default();
                activities.push(Self {
                    name,
                    id,
                    event_scripts,
                });
            }
        }

        Ok(activities)
    }
    pub fn save_activities(
        root: &Path,
        script_filename: &str,
        activities: &[Self],
    ) -> Result<(), error::Application> {
        for activity in activities {
            for (event, script_path) in &activity.event_scripts {
                let dest_dir = root.join(&activity.id).join(event.to_string());
                let dest_path = dest_dir.join(script_filename);

                fs::create_dir_all(&dest_dir).map_err(|_| error::SaveDataError {
                    activity: activity.name().clone(),
                    event: event.into(),
                    script_path: dest_path.to_string_lossy().into(),
                })?;

                if dest_path.exists() {
                    fs::remove_file(&dest_path).map_err(|_| error::SaveDataError {
                        activity: activity.name().clone(),
                        event: event.into(),
                        script_path: dest_path.to_string_lossy().into(),
                    })?;
                }

                symlink(script_path, &dest_path).map_err(|_| error::SaveDataError {
                    activity: activity.name().clone(),
                    event: event.into(),
                    script_path: dest_path.to_string_lossy().into(),
                })?;
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
    fn from_activity_data_contains_exactly() {
        let sample_data = r#"[Argument: a(ssssi) {[Argument: (ssssi) "some-id-a", "Activity A", "Ignore this", "ignore", 2], [Argument: (ssssi) "some-id-b", "activity B", "Stuff", "otherstuff", 13], [Argument: (ssssi) "some-id-d", "Long Named Activity", "Stuff", "otherstuff", 1], [Argument: (ssssi) "some-id-e", "Filing Taxes & Accounting", "Taxes", "taxes", 2]}]"#;

        let activities = Activity::from_activity_data(sample_data, ScriptMap::new()).unwrap();

        let actual: Vec<_> = activities
            .iter()
            .map(|a| (a.name().clone(), a.id().clone()))
            .collect();

        assert_that!(actual).contains_exactly([
            ("Activity A".to_string(), "some-id-a".to_string()),
            ("activity B".to_string(), "some-id-b".to_string()),
            ("Long Named Activity".to_string(), "some-id-d".to_string()),
            (
                "Filing Taxes & Accounting".to_string(),
                "some-id-e".to_string(),
            ),
        ]);
    }
    #[test]
    fn from_activity_data_populates_event_scripts() {
        let sample_data = r#"[Argument: a(ssssi) {[Argument: (ssssi) "some-id-a", "Activity A", "x", "y", 1], [Argument: (ssssi) "some-id-b", "Activity B", "x", "y", 1]}]"#;

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

        map.insert("some-id-a".into(), events_a.clone());
        map.insert("some-id-b".into(), events_b.clone());

        let activities = Activity::from_activity_data(sample_data, map).unwrap();

        assert_that!(activities.len()).is_equal_to(2);

        let a = activities.iter().find(|a| a.id() == "some-id-a").unwrap();
        assert_that!(a.event_scripts()).is_equal_to(&events_a);

        let b = activities.iter().find(|a| a.id() == "some-id-b").unwrap();
        assert_that!(b.event_scripts()).is_equal_to(&events_b);
    }
    #[test]
    fn load_scripts_reads_symlink_structure() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let activity_id = "some-id-x";
        let event_name = "activated";

        let activity_dir = root.join(activity_id).join(event_name);
        fs::create_dir_all(&activity_dir).unwrap();

        let actual_script = root.join("actual-script.sh");
        fs::write(&actual_script, "#!/bin/sh\necho Hello").unwrap();

        let symlink_path = activity_dir.join("kas-script.sh");
        symlink(&actual_script, &symlink_path).unwrap();

        let result = Activity::load_scripts(root, "kas-script.sh").unwrap();

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

        Activity::save_activities(root, "kas-script.sh", &[activity]).unwrap();

        let link_path = root.join("a-1/started/kas-script.sh");
        let meta = symlink_metadata(&link_path).unwrap();
        assert!(meta.file_type().is_symlink());

        let target = fs::read_link(link_path).unwrap();
        assert_eq!(target, source_script);
    }
}
