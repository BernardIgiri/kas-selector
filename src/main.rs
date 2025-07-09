#![deny(clippy::unwrap_used, clippy::expect_used)]
#![warn(clippy::all, clippy::nursery)]

mod activity;
mod config;
mod error;
mod locale;
mod shell_script_filename;

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

const STYLE: &str = r#"
.label {
    font-weight: bold;
}
"#;
const DEFAULT_KAS_PATH: &str = ".local/share/kactivitymanagerd/activities";
const DEFAULT_SCRIPT_FILENAME: &str = "activity_script.sh";
const KAS_HELP_URL: &str = "https://github.com/BernardIgiri/kas-selector";
const WINDOW_WIDTH: i32 = 500;
const WINDOW_HEIGHT: i32 = 260;

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
    spinner: gtk::Box,
}
#[derive(Debug)]
enum AppMsg {
    ChooseActivity(usize),
    ChooseScript(ActivityEvent),
    DeleteScript(ActivityEvent),
    ScriptChosen(PathBuf),
    ChooseScriptCancel,
    Exit,
    Help,
    Save,
    CloseSaveErrorDialog,
}
#[derive(Debug)]
enum AppCmd {
    SaveFinished(Result<(), error::Application>),
}
#[derive(Debug)]
struct AppInit {
    config: Config,
    activities: Vec<Activity>,
    lang: String,
}

impl AppModel {
    const fn can_save(&self) -> bool {
        self.is_dirty && !self.is_loading
    }
}

#[allow(clippy::expect_used)]
impl Component for AppModel {
    type Init = AppInit;
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = AppCmd;
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
        let locale =
            FluentLocale::try_new(&init.lang).expect("Failed to initialize localization: {e}");
        let open_dialog = OpenDialog::builder()
            .transient_for_native(&root)
            .launch(OpenDialogSettings {
                folder_mode: false,
                cancel_label: locale.text(locale::Key::Cancel, None),
                accept_label: locale.text(locale::Key::Open, None),
                create_folders: false,
                is_modal: true,
                filters: Vec::new(),
            })
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
        let provider = gtk::CssProvider::new();
        provider.load_from_string(STYLE);
        let display = gtk::gdk::Display::default().expect("Display should connect!");
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        root.set_default_width(WINDOW_WIDTH);
        root.set_default_height(WINDOW_HEIGHT);
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
                    set_tooltip: &model.locale.text(locale::Key::Activity, None),
                },

                #[name = "events_grid"]
                gtk::Grid {
                    set_row_spacing: 6,
                    set_column_spacing: 6,
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 6,
                    set_valign: gtk::Align::End,
                    set_vexpand: true,

                    #[name = "spinner"]
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 6,
                        set_visible: false,

                        gtk::Spinner {
                            set_spinning: true,
                        },
                        gtk::Label {
                            set_label: &model.locale.text(locale::Key::SavingData, None),
                        }
                    },
                    gtk::Box {
                        set_hexpand: true,
                    },
                    #[name = "quit_button"]
                    gtk::Button {
                        set_label: &model.locale.text(locale::Key::Quit, None),
                        set_size_request: (80, -1),
                    },
                    #[name = "save_button"]
                    gtk::Button {
                        set_label: &model.locale.text(locale::Key::Save, None),
                        set_sensitive: false,
                        set_size_request: (80, -1),
                    },
                    #[name = "help_button"]
                    gtk::Button::from_icon_name("help-about") {
                        set_tooltip: &model.locale.text(locale::Key::Help, None),
                    },
                }
            }
        }
        root.set_child(Some(&container));
        let mut path_labels = HashMap::new();
        let sender_clone = sender.clone();
        save_button.connect_clicked(move |_| {
            sender_clone.input(AppMsg::Save);
        });
        let sender_clone = sender.clone();
        quit_button.connect_clicked(move |_| {
            sender_clone.input(AppMsg::Exit);
        });
        let sender_clone = sender.clone();
        help_button.connect_clicked(move |_| {
            sender_clone.input(AppMsg::Help);
        });
        let sender_clone = sender.clone();
        save_error_dialog.connect_cancel_button_notify(move |_| {
            sender_clone.input(AppMsg::CloseSaveErrorDialog);
        });

        for (row, event) in ActivityEvent::iter().enumerate() {
            let script_path = model
                .activities
                .get(model.selected_activity_index)
                .and_then(|a| a.get_script(&event))
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            relm4::view! {
                event_label = gtk::Label {
                    set_label: &model.locale.text(event.as_key(), None),
                    set_halign: gtk::Align::Start,
                    add_css_class: "label"
                },
                path_label = gtk::Label {
                    set_label: &script_path,
                    set_hexpand: true,
                    set_halign: gtk::Align::Start,
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                },
                edit_button = gtk::Button::from_icon_name("edit"),
                delete_button = gtk::Button::from_icon_name("delete"),
            }
            let event_clone = event.clone();
            let sender_clone = sender.clone();
            edit_button.set_tooltip(&model.locale.text(locale::Key::Edit, None));
            edit_button.connect_clicked(move |_| {
                sender_clone.input(AppMsg::ChooseScript(event_clone.clone()));
            });

            let event_clone = event.clone();
            let sender_clone = sender.clone();
            delete_button.set_tooltip(&model.locale.text(locale::Key::Delete, None));
            delete_button.connect_clicked(move |_| {
                sender_clone.input(AppMsg::DeleteScript(event_clone.clone()));
            });

            events_grid.attach(&event_label, 0, row as i32, 1, 1);
            events_grid.attach(&path_label, 1, row as i32, 1, 1);
            events_grid.attach(&edit_button, 2, row as i32, 1, 1);
            events_grid.attach(&delete_button, 3, row as i32, 1, 1);

            path_labels.insert(event.clone(), path_label);
        }
        ComponentParts {
            model,
            widgets: Self::Widgets {
                root,
                path_labels,
                save_button,
                save_error_dialog,
                save_error_dialog_visible: false,
                spinner,
            },
        }
    }
    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        let activity = &self.activities[self.selected_activity_index];
        for (event, label) in widgets.path_labels.iter() {
            let path = activity
                .get_script(event)
                .map_or_else(|| "", |v| v.as_path().to_str().unwrap_or_default());
            label.set_text(path);
        }
        widgets.save_button.set_sensitive(self.can_save());
        if self.save_error_dialog_visible && !widgets.save_error_dialog_visible {
            widgets.save_error_dialog.show(Some(&widgets.root));
        }
        widgets.spinner.set_visible(self.is_loading);
    }
    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        dbg!(&message);
        let AppCmd::SaveFinished(result) = message;
        self.is_dirty = false;
        self.is_loading = false;
        if let Err(e) = result {
            eprintln!("Save failed due to: {e}");
            self.save_error_dialog_visible = true;
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
            AppMsg::Help => {
                if let Err(e) = open::that(KAS_HELP_URL) {
                    eprintln!("Could not show help due to: {e}");
                };
            }
            AppMsg::Save => {
                self.is_loading = true;
                let activities = self.activities.clone();
                let config = self.config.clone();
                sender.spawn_oneshot_command(move || {
                    AppCmd::SaveFinished(Activity::save_activities(
                        config.root_path(),
                        config.script_filename(),
                        &activities,
                    ))
                })
            }
            AppMsg::CloseSaveErrorDialog => {
                self.save_error_dialog_visible = false;
            }
        }
    }
}

fn get_env_lang() -> String {
    for var in ["LANGUAGE", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() {
                return val.split('.').next().unwrap_or("en_US").replace('_', "-");
            }
        }
    }
    "en-US".into()
}

#[allow(clippy::expect_used)]
fn main() {
    let root_path = std::env::var("KAS_ROOT").map_or_else(
        |_| PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(DEFAULT_KAS_PATH),
        PathBuf::from,
    );
    let script_filename = std::env::var("KAS_SCRIPT_NAME")
        .unwrap_or_else(|_| DEFAULT_SCRIPT_FILENAME.into())
        .parse()
        .expect("Script filename validation check.");
    let config = Config::new(root_path, script_filename);
    let activities = Activity::from_env(config.root_path(), config.script_filename())
        .expect("Loading activity data.");
    let lang = get_env_lang();
    relm4::RelmApp::new("kas-selector").run::<AppModel>(AppInit {
        config,
        activities,
        lang,
    });
}
