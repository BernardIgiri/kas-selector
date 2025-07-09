use fluent_bundle::{FluentArgs, FluentResource, concurrent::FluentBundle};
use indexmap::IndexSet;
use std::{env, fmt::Debug, fs, path::PathBuf, sync::Arc};
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};
use unic_langid::LanguageIdentifier;

use crate::error;

fn locale_root_prefix(p: &str) -> String {
    let p = p.trim();
    format!("{p}/kas-selector/locales")
}

fn locale_roots() -> Vec<String> {
    let mut seen = IndexSet::from(["locales".to_string()]);
    seen.extend(["/usr/local/share", "/usr/share"].map(locale_root_prefix));
    seen.extend(
        env::var("XDG_DATA_DIRS")
            .unwrap_or_default()
            .split(':')
            .map(locale_root_prefix),
    );
    seen.into_iter().collect()
}

#[derive(EnumString, EnumIter, Display, Debug)]
#[strum(serialize_all = "kebab-case")]
pub enum Key {
    Title,
    EventActivated,
    EventDeactivated,
    EventStarted,
    EventStopped,
    Open,
    Cancel,
    Quit,
    Save,
    Edit,
    Help,
    Delete,
    ErrorSaveFailed,
    SavingData,
    Activity,
}

#[derive(Clone)]
pub struct FluentLocale {
    bundle: Arc<FluentBundle<FluentResource>>,
}

impl FluentLocale {
    pub fn try_new(lang: &str) -> Result<Self, error::Application> {
        let locale_roots = locale_roots();
        let lang_id: LanguageIdentifier = lang.parse().map_err(|_| error::BadInitData {
            category: "Language invalid",
            value: lang.into(),
        })?;
        let (source, path) = locale_roots
            .iter()
            .map(|root| {
                let path = PathBuf::new()
                    .join(root)
                    .join(lang_id.to_string())
                    .join("main.ftl");
                (
                    fs::read_to_string(&path),
                    path.to_string_lossy().to_string(),
                )
            })
            .find_map(|(result, path)| result.ok().map(|source| (source, path)))
            .ok_or_else(|| error::BadInitData {
                category: "Fluent file not found",
                value: locale_roots.join(", "),
            })?;
        let resource = FluentResource::try_new(source).map_err(|_| error::BadInitData {
            category: "Fluent syntax error",
            value: path.clone(),
        })?;

        let mut bundle = FluentBundle::new_concurrent(vec![lang_id]);
        bundle
            .add_resource(resource)
            .map_err(|_| error::BadInitData {
                category: "Fluent bundle",
                value: path.clone(),
            })?;
        for key in Key::iter() {
            if !bundle.has_message(key.to_string().as_str()) {
                return Err(error::BadInitData {
                    category: "Fluent key missing",
                    value: key.to_string(),
                });
            }
        }
        Ok(Self {
            bundle: Arc::new(bundle),
        })
    }

    pub fn text(&self, key: Key, args: Option<&FluentArgs>) -> String {
        #[allow(clippy::expect_used)]
        let pattern = self
            .bundle
            .get_message(key.to_string().as_str())
            .and_then(|msg| msg.value())
            .expect("All keys were validated during construction!");
        self.bundle
            .format_pattern(pattern, args, &mut vec![])
            .to_string()
    }
}

impl Debug for FluentLocale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FluentLocal")
    }
}

// Allowed in tests
#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use asserting::prelude::*;
    use temp_env::with_var;

    use super::*;

    #[test]
    fn us_translation_is_valid() {
        let locale = FluentLocale::try_new("en-US");
        assert_that!(locale)
            .is_ok()
            .satisfies_with_message("Title found in locale", |l| {
                !l.clone().unwrap().text(Key::Title, None).is_empty()
            });
    }
    #[test]
    fn locale_roots_with_custom_xdg_dirs() {
        with_var("XDG_DATA_DIRS", Some("/one:/two:/usr/share"), || {
            let roots = locale_roots();
            assert!(
                roots.contains(&"locales".to_string()),
                "Missing dev path: 'locales'"
            );
            assert!(
                roots.contains(&"/one/kas-selector/locales".to_string()),
                "Missing transformed XDG path: /one/kas-selector/locales"
            );
            assert!(
                roots.contains(&"/two/kas-selector/locales".to_string()),
                "Missing transformed XDG path: /two/kas-selector/locales"
            );
            assert!(
                roots.contains(&"/usr/local/share/kas-selector/locales".to_string()),
                "Missing fallback path: /usr/local/share/kas-selector/locales"
            );
            assert!(
                roots.contains(&"/usr/share/kas-selector/locales".to_string()),
                "Missing fallback path: /usr/share/kas-selector/locales"
            );
            let unique: HashSet<_> = roots.iter().collect();
            assert_eq!(
                roots.len(),
                unique.len(),
                "Duplicate entries found in locale_roots"
            );
        });
    }
    #[test]
    fn locale_roots_with_empty_env() {
        with_var("XDG_DATA_DIRS", Option::<&str>::None, || {
            let roots = locale_roots();
            assert!(roots.contains(&"locales".to_string()));
            assert!(roots.contains(&"/usr/local/share/kas-selector/locales".to_string()));
            assert!(roots.contains(&"/usr/share/kas-selector/locales".to_string()));
        });
    }
    #[test]
    fn locale_roots_is_in_priority_order() {
        with_var("XDG_DATA_DIRS", Some("/one:/two:/three"), || {
            let roots = locale_roots();
            let order = [
                "locales",
                "/usr/local/share",
                "/usr/share",
                "/one",
                "/two",
                "/three",
            ];
            for (i, prefix) in order.iter().enumerate() {
                assert_that!(roots[i].clone()).starts_with(prefix.to_string());
            }
        });
    }
}
