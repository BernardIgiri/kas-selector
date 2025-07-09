use fluent_bundle::{FluentArgs, FluentResource, concurrent::FluentBundle};
use std::{fmt::Debug, fs, sync::Arc};
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};
use unic_langid::LanguageIdentifier;

use crate::error;

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
        let lang_id: LanguageIdentifier = lang.parse().map_err(|_| error::BadInitData {
            category: "Language",
            value: lang.into(),
        })?;
        let path = format!("locales/{lang_id}/main.ftl");
        let source = fs::read_to_string(&path).map_err(|_| error::BadInitData {
            category: "Fluent file",
            value: path.clone(),
        })?;
        let resource = FluentResource::try_new(source).map_err(|_| error::BadInitData {
            category: "Fluent syntax",
            value: path.clone(),
        })?;

        let mut bundle = FluentBundle::new_concurrent(vec![lang_id]);
        bundle
            .add_resource(resource)
            .map_err(|_| error::BadInitData {
                category: "Fluent bundle",
                value: path,
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
