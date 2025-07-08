#![deny(clippy::unwrap_used, clippy::expect_used)]
#![warn(clippy::all, clippy::nursery)]

mod activity;
mod error;
mod locale;

use activity::{Activity, ActivityEvent};
use gtk::prelude::*;
use locale::FluentLocale;
use relm4::prelude::*;
use relm4_components::open_dialog::{
    OpenDialog, OpenDialogMsg, OpenDialogResponse, OpenDialogSettings,
};
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;
use strum::IntoEnumIterator;

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

#[derive(Debug)]
struct AppModel {
    activities: Vec<Activity>,
    selected_activity_index: usize,
    locale: FluentLocale,
    open_dialog: Controller<OpenDialog>,
    pending_event: ActivityEvent,
    path_labels: HashMap<ActivityEvent, gtk::Label>,
}

#[derive(Debug)]
enum AppMsg {
    ChooseActivity(usize),
    ChooseScript(ActivityEvent),
    DeleteScript(ActivityEvent),
    ScriptChosen(PathBuf),
    ChooseScriptCancel,
}

#[relm4::component]
impl SimpleComponent for AppModel {
    type Init = Vec<Activity>;
    type Input = AppMsg;
    type Output = ();

    view! {
        #[root]
        window = gtk::ApplicationWindow {
            set_title: Some(&model.locale.text(locale::Key::Title, None)),
            set_default_width: 600,
            set_default_height: 400,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 12,
                set_margin_all: 12,

                #[name = "dropdown"]
                gtk::DropDown::from_strings(&model.activities.iter().map(|a| a.name().as_str()).collect::<Vec<_>>()) {
                    connect_selected_notify[sender] => move |dropdown| {
                        sender.input(AppMsg::ChooseActivity(dropdown.selected() as usize))
                    },
                    set_selected: model.selected_activity_index as u32,
                },

                #[name = "events_box"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 6,
                },
            }
        }
    }

    fn init(
        activities: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let lang = &get_kde_preferred_language();
        let locale = FluentLocale::try_new(lang).unwrap_or_else(|e| {
            eprintln!("Failed to initialize localization: {e}");
            std::process::exit(1);
        });
        let open_dialog = OpenDialog::builder()
            .transient_for_native(&root)
            .launch(OpenDialogSettings::default())
            .forward(sender.input_sender(), |response| match response {
                OpenDialogResponse::Accept(path) => AppMsg::ScriptChosen(path),
                OpenDialogResponse::Cancel => AppMsg::ChooseScriptCancel,
            });

        let mut model = Self {
            activities,
            selected_activity_index: 0,
            locale,
            open_dialog,
            pending_event: ActivityEvent::Activated,
            path_labels: HashMap::new(),
        };
        let widgets = view_output!();

        for event in ActivityEvent::iter() {
            let label_text = model.locale.text(event.as_key(), None);
            let script_path = model
                .activities
                .get(model.selected_activity_index)
                .and_then(|a| a.event_scripts().get(&event))
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);

            let label = gtk::Label::new(Some(&label_text));
            label.set_xalign(0.0);
            row.append(&label);

            let path_label = gtk::Label::new(Some(&script_path));
            path_label.set_xalign(0.0);
            path_label.set_hexpand(true);
            row.append(&path_label);
            model.path_labels.insert(event.clone(), path_label);

            let button = gtk::Button::from_icon_name("edit");
            let event_clone = event.clone();
            let sender_clone = sender.clone();
            button.connect_clicked(move |_| {
                sender_clone.input(AppMsg::ChooseScript(event_clone.clone()));
            });
            row.append(&button);

            let button = gtk::Button::from_icon_name("delete");
            let event_clone = event.clone();
            let sender_clone = sender.clone();
            button.connect_clicked(move |_| {
                sender_clone.input(AppMsg::DeleteScript(event_clone.clone()));
            });
            row.append(&button);

            widgets.events_box.append(&row);
        }
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: AppMsg, _sender: ComponentSender<Self>) {
        dbg!(&msg);
        match msg {
            AppMsg::ChooseActivity(index) => {
                self.selected_activity_index = index;
            }
            AppMsg::ChooseScript(event) => {
                self.pending_event = event;
                self.open_dialog.emit(OpenDialogMsg::Open);
            }
            AppMsg::ScriptChosen(path_buf) => {
                self.activities[self.selected_activity_index]
                    .set_script(self.pending_event.clone(), path_buf.clone());
                #[allow(clippy::expect_used)]
                self.path_labels
                    .get(&self.pending_event)
                    .expect("Path labels for all events should exist.")
                    .set_text(path_buf.to_string_lossy().as_ref());
            }
            AppMsg::ChooseScriptCancel => {}
            AppMsg::DeleteScript(activity_event) => {
                self.activities[self.selected_activity_index].delete_script(activity_event.clone());
                #[allow(clippy::expect_used)]
                self.path_labels
                    .get(&activity_event)
                    .expect("Path labels for all events should exist.")
                    .set_text("");
            }
        }
    }
}

fn main() {
    let root_path = PathBuf::from("/home/bigiri/.local/share/kactivitymanagerd/activities");
    let script_filename = "activity_script";
    let activities = Activity::from_env(&root_path, script_filename).unwrap_or_else(|e| {
        eprintln!("Failed to load activity data: {e}");
        std::process::exit(1);
    });
    relm4::RelmApp::new("kas-selector").run::<AppModel>(activities);
}
