use fluent_bundle::{FluentArgs, FluentResource, concurrent::FluentBundle};
use fluent_langneg::{NegotiationStrategy, convert_vec_str_to_langids_lossy, negotiate_languages};
use indexmap::IndexSet;
use std::{env, fmt::Debug, fs, path::PathBuf, sync::Arc};
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};
use unic_langid::LanguageIdentifier;

use crate::error;

pub const DEFAULT_LOCALE: &str = "en-US";
pub const AVAILABLE_LOCALES: [&str; 7] = ["ar", "de", "en-US", "es", "fr", "ru", "zh"];

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

fn negotiated_lang_from_str(lang: &str) -> Result<LanguageIdentifier, error::Application> {
    let lang_id: LanguageIdentifier = lang.parse().map_err(|_| error::InvalidValue {
        category: "Language invalid",
        value: lang.into(),
    })?;
    #[allow(clippy::expect_used)]
    let default = DEFAULT_LOCALE
        .parse()
        .expect("Default language id should be parseable.");
    let available = convert_vec_str_to_langids_lossy(AVAILABLE_LOCALES);
    Ok(negotiate_languages(
        &[&lang_id],
        &available,
        Some(&default),
        NegotiationStrategy::Lookup,
    )
    .first()
    .cloned()
    .ok_or_else(|| error::UnsupportedValue {
        category: "langauge",
        value: lang.to_string(),
    })?
    .clone())
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
        let lang_id = negotiated_lang_from_str(lang)?;
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
            .ok_or_else(|| error::UnsupportedValue {
                category: "Fluent file",
                value: locale_roots.join(", "),
            })?;
        let resource = FluentResource::try_new(source).map_err(|_| error::InvalidValue {
            category: "Fluent syntax error",
            value: path.clone(),
        })?;

        let mut bundle = FluentBundle::new_concurrent(vec![lang_id]);
        bundle
            .add_resource(resource)
            .map_err(|_| error::InvalidValue {
                category: "Fluent bundle",
                value: path.clone(),
            })?;
        for key in Key::iter() {
            if !bundle.has_message(key.to_string().as_str()) {
                return Err(error::UnsupportedValue {
                    category: "Fluent key",
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
    use asserting::prelude::*;
    use temp_env::with_var;

    use super::*;

    #[test]
    fn available_locales_match_folder() {
        let locale_folders = fs::read_dir("locales").unwrap().filter_map(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();
            match path.file_name().and_then(|n| n.to_str()) {
                Some(name) if path.is_dir() => Some(name.to_string()),
                _ => None,
            }
        });
        assert_that!(AVAILABLE_LOCALES).contains_exactly_in_any_order(locale_folders);
    }
    #[test]
    fn all_translations_are_valid() {
        for lang in AVAILABLE_LOCALES {
            assert_that!(lang)
                .described_as(lang)
                .satisfies_with_message("Is retrievable from negotiation", |lang| {
                    negotiated_lang_from_str(lang).unwrap()
                        == lang.parse::<LanguageIdentifier>().unwrap()
                })
                .extracting(FluentLocale::try_new)
                .is_ok()
                .extracting(|locale| locale.unwrap())
                .satisfies_with_message("Title found in locale", |locale| {
                    !locale.text(Key::Title, None).is_empty()
                });
        }
    }
    #[test]
    fn locale_roots_with_custom_xdg_dirs() {
        with_var("XDG_DATA_DIRS", Some("/one:/two:/usr/share"), || {
            let roots = locale_roots();
            assert_that!(&roots)
                .contains_all_of([
                    "locales",
                    "/one/kas-selector/locales",
                    "/two/kas-selector/locales",
                    "/usr/local/share/kas-selector/locales",
                    "/usr/share/kas-selector/locales",
                ])
                .described_as("no duplicates")
                .contains_only_once(&roots);
        });
    }
    #[test]
    fn locale_roots_with_empty_env() {
        with_var("XDG_DATA_DIRS", Option::<&str>::None, || {
            assert_that!(locale_roots()).contains_all_of([
                "locales".to_string(),
                "/usr/local/share/kas-selector/locales".to_string(),
                "/usr/share/kas-selector/locales".to_string(),
            ]);
        });
    }
    #[test]
    fn locale_roots_is_in_priority_order() {
        with_var("XDG_DATA_DIRS", Some("/one:/two:/three"), || {
            let root_list = locale_roots();
            let order_list = [
                "locales",
                "/usr/local/share",
                "/usr/share",
                "/one",
                "/two",
                "/three",
            ];
            for (i, (root, prefix)) in root_list.iter().zip(order_list).enumerate() {
                assert_that!(root)
                    .described_as(format!("index: {i}"))
                    .starts_with(prefix);
            }
        });
    }
}
