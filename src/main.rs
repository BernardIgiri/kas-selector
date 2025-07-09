#![deny(clippy::unwrap_used, clippy::expect_used)]
#![warn(clippy::all, clippy::nursery)]

mod activity;
mod config;
mod error;
mod locale;

use activity::{Activity, ActivityEvent};
use config::Config;
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
    config: Config,
    activities: Vec<Activity>,
    selected_activity_index: usize,
    locale: FluentLocale,
    open_dialog: Controller<OpenDialog>,
    pending_event: ActivityEvent,
    is_dirty: bool,
    is_loading: bool,
    save_error_dialog_visible: bool,
}
#[derive(Debug)]
struct AppWidgets {
    root: gtk::Window,
    path_labels: HashMap<ActivityEvent, gtk::Label>,
    save_button: gtk::Button,
    save_error_dialog: gtk::AlertDialog,
    save_error_dialog_visible: bool,
}
#[derive(Debug)]
enum AppMsg {
    ChooseActivity(usize),
    ChooseScript(ActivityEvent),
    DeleteScript(ActivityEvent),
    ScriptChosen(PathBuf),
    ChooseScriptCancel,
    Exit,
    SaveStarted,
    SaveFinished(Result<(), error::Application>),
    CloseSaveErrorDialog,
}
#[derive(Debug)]
struct AppInit {
    config: Config,
    activities: Vec<Activity>,
}

impl Component for AppModel {
    type Init = AppInit;
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = AppMsg;
    type Root = gtk::Window;
    type Widgets = AppWidgets;

    fn init_root() -> Self::Root {
        gtk::Window::default()
    }

    fn init(
        init: Self::Init,
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
        let model = Self {
            config: init.config,
            activities: init.activities,
            selected_activity_index: 0,
            locale,
            open_dialog,
            pending_event: ActivityEvent::Activated,
            is_dirty: false,
            is_loading: false,
            save_error_dialog_visible: false,
        };
        root.set_default_width(600);
        root.set_default_height(400);
        root.set_title(Some(model.locale.text(locale::Key::Title, None).as_str()));
        relm4::view! {
            save_error_dialog = gtk::AlertDialog {
                set_modal: true,
                set_message: &model.locale.text(locale::Key::ErrorSaveFailed, None),
            },
            container = gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 12,
                set_margin_all: 12,

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

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 6,
                    set_halign: gtk::Align::End,
                    set_valign: gtk::Align::End,
                    set_vexpand: true,

                    #[name = "exit_button"]
                    gtk::Button::with_label(&model.locale.text(locale::Key::Exit, None)),
                    #[name = "save_button"]
                    gtk::Button::with_label(&model.locale.text(locale::Key::Save, None)),
                }
            }
        }
        root.set_child(Some(&container));
        let mut path_labels = HashMap::new();
        let sender_clone = sender.clone();
        save_button.connect_clicked(move |_| {
            sender_clone.input(AppMsg::SaveStarted);
        });
        let sender_clone = sender.clone();
        exit_button.connect_clicked(move |_| {
            sender_clone.input(AppMsg::Exit);
        });
        let sender_clone = sender.clone();
        save_error_dialog.connect_cancel_button_notify(move |_| {
            sender_clone.input(AppMsg::CloseSaveErrorDialog);
        });

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
            path_labels.insert(event.clone(), path_label);

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

            events_box.append(&row);
        }
        ComponentParts {
            model,
            widgets: Self::Widgets {
                root,
                path_labels,
                save_button,
                save_error_dialog,
                save_error_dialog_visible: false,
            },
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        let activity = &self.activities[self.selected_activity_index];
        for (event, label) in widgets.path_labels.iter() {
            let path = activity
                .event_scripts()
                .get(event)
                .map_or_else(|| "", |v| v.as_path().to_str().unwrap_or_default());
            label.set_text(path);
        }
        let can_save = match (self.is_dirty, self.is_loading) {
            (_, true) => false,
            (true, _) => true,
            _ => false,
        };
        widgets.save_button.set_sensitive(can_save);
        if self.save_error_dialog_visible && !widgets.save_error_dialog_visible {
            widgets.save_error_dialog.show(Some(&widgets.root));
        }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        dbg!(&message);
        match message {
            AppMsg::ChooseActivity(index) => {
                self.selected_activity_index = index;
            }
            AppMsg::ChooseScript(event) => {
                self.pending_event = event;
                self.open_dialog.emit(OpenDialogMsg::Open);
            }
            AppMsg::ScriptChosen(path_buf) => {
                self.is_dirty = true;
                self.activities[self.selected_activity_index]
                    .set_script(self.pending_event.clone(), path_buf);
            }
            AppMsg::ChooseScriptCancel => {}
            AppMsg::DeleteScript(activity_event) => {
                self.is_dirty = true;
                self.activities[self.selected_activity_index].delete_script(activity_event);
            }
            AppMsg::Exit => {
                relm4::main_application().quit();
            }
            AppMsg::SaveStarted => {
                self.is_loading = true;
                let activities = self.activities.clone();
                let config = self.config.clone();
                sender.oneshot_command(async move {
                    AppMsg::SaveFinished(Activity::save_activities(
                        config.root_path(),
                        config.script_filename(),
                        &activities,
                    ))
                })
            }
            AppMsg::SaveFinished(result) => {
                self.is_dirty = false;
                self.is_loading = false;
                if let Err(e) = result {
                    eprintln!("{e}");
                    self.save_error_dialog_visible = true;
                }
            }
            AppMsg::CloseSaveErrorDialog => {
                self.save_error_dialog_visible = false;
            }
        }
    }
}

fn main() {
    let root_path = PathBuf::from("/home/bigiri/.local/share/kactivitymanagerd/activities");
    let script_filename = "activity_script";
    let config = Config::new(root_path, script_filename.into());
    let activities = Activity::from_env(config.root_path(), config.script_filename())
        .unwrap_or_else(|e| {
            eprintln!("Failed to load activity data: {e}");
            std::process::exit(1);
        });
    relm4::RelmApp::new("kas-selector").run::<AppModel>(AppInit { config, activities });
}
