use derive_new::new;
use fluent_resmgr::ResourceManager;
use gtk::prelude::*;
use relm4::prelude::*;
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::LazyLock;

use fluent_bundle::FluentArgs;

use unic_langid::LanguageIdentifier;

// KDE-style locale resolution
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

static LANG_ID: LazyLock<LanguageIdentifier> = LazyLock::new(|| {
    get_kde_preferred_language()
        .parse()
        .unwrap_or_else(|_| "en-US".parse().unwrap())
});

#[derive(new)]
struct FluentLocale(ResourceManager);

impl Debug for FluentLocale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FluentLocal")
    }
}

impl FluentLocale {
    pub fn tr(&self, key: &str, args: Option<&FluentArgs>) -> String {
        let bundle = self.0.get_bundle(vec![LANG_ID.clone()], vec![]).unwrap();
        let pattern = match bundle.get_message(key).and_then(|msg| msg.value()) {
            Some(p) => p,
            None => {
                eprintln!("⚠️ Missing translation key: {key}");
                return key.to_string();
            }
        };
        bundle
            .format_pattern(pattern, args, &mut vec![])
            .to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Event {
    Activated,
    Deactivated,
    Started,
    Stopped,
}

impl Event {
    fn all() -> &'static [Event] {
        use Event::*;
        &[Activated, Deactivated, Started, Stopped]
    }

    fn as_key(&self) -> &'static str {
        match self {
            Event::Activated => "event-activated",
            Event::Deactivated => "event-deactivated",
            Event::Started => "event-started",
            Event::Stopped => "event-stopped",
        }
    }
}

#[derive(Debug, Clone)]
struct Activity {
    name: String,
    id: String,
    event_scripts: HashMap<Event, Option<PathBuf>>, // None = no script assigned
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
        let locale = FluentLocale::new(ResourceManager::new(
            "locales/{locale}/main.ftl".to_string(),
        ));
        let model = AppModel { activities, locale };
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 12);

        let mut rows = vec![];

        for (i, activity) in model.activities.iter().enumerate() {
            let frame = gtk::Frame::new(Some(&activity.name));
            let inner = gtk::Box::new(gtk::Orientation::Vertical, 6);

            for event in Event::all() {
                let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 6);

                let label = gtk::Label::new(Some(&model.locale.tr(event.as_key(), None)));
                label.set_xalign(0.0);

                let path_label = gtk::Label::new(
                    activity
                        .event_scripts
                        .get(event)
                        .and_then(|p| p.as_ref())
                        .map(|p| p.to_string_lossy().to_string())
                        .as_deref(),
                );
                path_label.set_xalign(0.0);
                path_label.set_hexpand(true);

                let button = gtk::Button::with_label(&model.locale.tr("choose-script", None));
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

    relm4::RelmApp::new("kde-script-binder").run::<AppModel>(mock_data);
}

// fn open_file_dialog(parent: &Window) -> Option<PathBuf> {
//     let dialog = FileChooserDialog::new(
//         Some("Choose a script"),
//         Some(parent),
//         FileChooserAction::Open,
//         &[("Cancel", ResponseType::Cancel), ("Open", ResponseType::Accept)],
//     );

//     let filter = FileFilter::new();
//     filter.add_pattern("*.sh");
//     filter.set_name(Some("Shell scripts"));
//     dialog.add_filter(&filter);

//     let result = if dialog.run() == ResponseType::Accept {
//         dialog.file().and_then(|f| f.path())
//     } else {
//         None
//     };

//     dialog.close();
//     result
// }
