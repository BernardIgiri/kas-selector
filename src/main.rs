#![deny(clippy::unwrap_used, clippy::expect_used)]
#![warn(clippy::all, clippy::nursery)]

mod error;
mod fluent;

use fluent::FluentLocale;
use gtk::{StringList, prelude::*};
use relm4::prelude::*;
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;
use strum::{EnumIter, IntoEnumIterator};

fn get_kde_preferred_language() -> String {
    std::env::var("LANGUAGE")
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_else(|_| "en_US.UTF-8".into())
        .split('.')
        .next()
        .unwrap_or("en_US")
        .replace('_', "-")
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumIter)]
enum Event {
    Activated,
    Deactivated,
    Started,
    Stopped,
}

impl Event {
    pub const fn as_key(&self) -> fluent::Key {
        use fluent::Key as K;
        match self {
            Self::Activated => K::EventActivated,
            Self::Deactivated => K::EventDeactivated,
            Self::Started => K::EventStarted,
            Self::Stopped => K::EventStopped,
        }
    }
}

#[derive(Debug, Clone)]
struct Activity {
    name: String,
    id: String,
    event_scripts: HashMap<Event, Option<PathBuf>>,
}

#[derive(Debug)]
struct AppModel {
    activities: Vec<Activity>,
    locale: FluentLocale,
}

#[derive(Debug)]
enum AppMsg {
    ChooseScript(usize, Event),
    ScriptChosen(usize, Event, PathBuf),
}

struct AppWidgets {
    rows: Vec<gtk::Box>,
}

impl Component for AppModel {
    fn init_root() -> Self::Root {
        gtk::ApplicationWindow::builder()
            .default_width(600)
            .default_height(400)
            .build()
    }
    type CommandOutput = ();
    type Init = Vec<Activity>;
    type Input = AppMsg;
    type Output = (); // No output for now
    type Root = gtk::ApplicationWindow;
    type Widgets = AppWidgets;

    fn init(
        activities: Self::Init,
        window: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let lang = &get_kde_preferred_language();
        let locale = FluentLocale::try_new(lang).unwrap_or_else(|e| {
            eprintln!("Failed to initialize localization: {e}");
            std::process::exit(1);
        });
        let model = Self { activities, locale };
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 12);

        let mut rows = vec![];

        for (i, activity) in model.activities.iter().enumerate() {
            let frame = gtk::Frame::new(Some(&activity.name));
            let inner = gtk::Box::new(gtk::Orientation::Vertical, 6);

            for event in Event::iter() {
                let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 6);

                let label = gtk::Label::new(Some(&model.locale.text(event.as_key(), None)));
                label.set_xalign(0.0);

                let path_label = gtk::Label::new(
                    activity
                        .event_scripts
                        .get(&event)
                        .and_then(|p| p.as_ref())
                        .map(|p| p.to_string_lossy().to_string())
                        .as_deref(),
                );
                path_label.set_xalign(0.0);
                path_label.set_hexpand(true);

                let button =
                    gtk::Button::with_label(&model.locale.text(fluent::Key::ChooseScript, None));
                let event_clone = event.clone();
                let sender_clone = sender.clone();
                button.connect_clicked(move |_| {
                    sender_clone.input(AppMsg::ChooseScript(i, event_clone.clone()));
                });

                hbox.append(&label);
                hbox.append(&path_label);
                hbox.append(&button);
                inner.append(&hbox);
            }

            frame.set_child(Some(&inner));
            vbox.append(&frame);
            rows.push(inner);
        }

        window.set_child(Some(&vbox));
        window.set_title(Some(&model.locale.text(fluent::Key::Title, None)));

        let widgets = AppWidgets { rows };
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: AppMsg, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            AppMsg::ChooseScript(activity_idx, event) => {
                println!("Open file dialog for activity {activity_idx} event {event:?}");
                // Hook up FileChooserDialog in future
            }
            AppMsg::ScriptChosen(activity_idx, event, path) => {
                if let Some(activity) = self.activities.get_mut(activity_idx) {
                    activity.event_scripts.insert(event, Some(path));
                }
            }
        }
    }
}

fn main() {
    let mock_data = vec![
        Activity {
            name: "Writing".to_string(),
            id: "activity-123".to_string(),
            event_scripts: HashMap::new(),
        },
        Activity {
            name: "Gaming".to_string(),
            id: "activity-456".to_string(),
            event_scripts: HashMap::new(),
        },
    ];

    relm4::RelmApp::new("kas-selector").run::<AppModel>(mock_data);
}
