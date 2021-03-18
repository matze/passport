use crate::application::Application;
use crate::config::{APP_ID, PROFILE};
use crate::storage;
use glib::clone;
use glib::signal::Inhibit;
use gtk::subclass::prelude::*;
use gtk::{self, prelude::*};
use gtk::{gio, glib, CompositeTemplate};
use gtk_macros::action;
use log::warn;
use std::thread;

pub enum Message {
    ClearEntry,
    UpdateEntry(String, String),
    HideInfoBar,
}

mod imp {
    use super::*;

    #[derive(CompositeTemplate)]
    #[template(resource = "/net/bloerg/Passport/window.ui")]
    pub struct ApplicationWindow {
        pub settings: gio::Settings,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub store: TemplateChild<gio::ListStore>,
        #[template_child]
        pub selection: TemplateChild<gtk::SingleSelection>,
        #[template_child]
        pub password: TemplateChild<gtk::PasswordEntry>,
        #[template_child]
        pub metadata: TemplateChild<gtk::TextView>,
        #[template_child]
        pub metadata_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub info_bar: TemplateChild<gtk::InfoBar>,
        #[template_child]
        pub info_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub entry_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub add_password_cancel_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ApplicationWindow {
        const NAME: &'static str = "ApplicationWindow";
        type Type = super::ApplicationWindow;
        type ParentType = gtk::ApplicationWindow;

        fn new() -> Self {
            Self {
                stack: TemplateChild::default(),
                settings: gio::Settings::new(APP_ID),
                search_bar: TemplateChild::default(),
                search_entry: TemplateChild::default(),
                store: TemplateChild::default(),
                selection: TemplateChild::default(),
                password: TemplateChild::default(),
                metadata: TemplateChild::default(),
                metadata_revealer: TemplateChild::default(),
                info_bar: TemplateChild::default(),
                info_label: TemplateChild::default(),
                entry_label: TemplateChild::default(),
                add_password_cancel_button: TemplateChild::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        // You must call `Widget`'s `init_template()` within `instance_init()`.
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ApplicationWindow {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let builder = gtk::Builder::from_resource("/net/bloerg/Passport/shortcuts.ui");
            let shortcuts = builder.get_object("shortcuts").unwrap();
            obj.set_help_overlay(Some(&shortcuts));

            // Devel Profile
            if PROFILE == "Devel" {
                obj.get_style_context().add_class("devel");
            }

            // load latest window state
            obj.load_window_size();

            let storage = storage::Storage::new().unwrap();

            for entry in storage.entries() {
                // We re-use GtkLabel which is lame but I am too stupid to define my own simple
                // GObject subclass.
                let label = gtk::Label::new(Some(&entry));
                label.set_halign(gtk::Align::Start);
                self.store.append(&label);
            }
        }
    }

    impl WindowImpl for ApplicationWindow {
        // save window state on delete event
        fn close_request(&self, obj: &Self::Type) -> Inhibit {
            if let Err(err) = obj.save_window_size() {
                warn!("Failed to save window state, {}", &err);
            }
            Inhibit(false)
        }
    }

    impl WidgetImpl for ApplicationWindow {}
    impl ApplicationWindowImpl for ApplicationWindow {}
}

glib::wrapper! {
    pub struct ApplicationWindow(ObjectSubclass<imp::ApplicationWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, @implements gio::ActionMap, gio::ActionGroup;
}

impl ApplicationWindow {
    pub fn new(app: &Application) -> Self {
        let window: Self = glib::Object::new(&[]).expect("Failed to create ApplicationWindow");
        window.set_application(Some(app));

        // Set icons for shell
        gtk::Window::set_default_icon_name(APP_ID);

        let (sender, receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        // Handle background requests
        receiver.attach(None,
            clone!(@weak window as win => move |message| {
                match message {
                    Message::ClearEntry => {
                        let password = &imp::ApplicationWindow::from_instance(&win).password;
                        password.set_text("");

                        let revealer = &imp::ApplicationWindow::from_instance(&win).metadata_revealer;
                        revealer.set_reveal_child(false);

                        let metadata = &imp::ApplicationWindow::from_instance(&win).metadata;
                        let buffer = metadata.get_buffer();
                        buffer.set_text("");
                    },
                    Message::UpdateEntry(password_text, metadata_text) => {
                        let password = &imp::ApplicationWindow::from_instance(&win).password;
                        password.set_text(&password_text);

                        let metadata = &imp::ApplicationWindow::from_instance(&win).metadata;
                        let buffer = metadata.get_buffer();
                        buffer.set_text(&metadata_text);

                        if metadata_text.len() > 0 {
                            let revealer = &imp::ApplicationWindow::from_instance(&win).metadata_revealer;
                            revealer.set_reveal_child(true);
                        }
                    },
                    Message::HideInfoBar => {
                        let info_bar = &imp::ApplicationWindow::from_instance(&win).info_bar;
                        info_bar.set_revealed(false);
                    },
                }

                glib::Continue(true)
            })
        );

        let selection = &imp::ApplicationWindow::from_instance(&window).selection.clone();

        selection.connect_selection_changed(
            clone!(@strong sender => move |selection, _, _| {
                if let Some(item) = selection.get_object(selection.get_selected()) {
                    let label = item.downcast::<gtk::Label>().unwrap();
                    let entry = label.get_text().clone();
                    let sender = sender.clone();

                    thread::spawn(move || {
                        let _ = sender.send(Message::ClearEntry);

                        let storage = storage::Storage::new().unwrap();
                        let entry = storage.decrypt(&entry).unwrap();
                        let _ = sender.send(Message::UpdateEntry(entry.password, entry.metadata));
                    });
                }
            }),
        );

        let button = &imp::ApplicationWindow::from_instance(&window).add_password_cancel_button.clone();

        button.connect_clicked(
            clone!(@weak window as win => move |_| {
                println!("clicked!");
                let stack = &imp::ApplicationWindow::from_instance(&win).stack;
                stack.set_visible_child_name("main_page");
            })
        );

        action!(
            window,
            "add",
            clone!(@weak window as win => move |_, _| {
                let stack = &imp::ApplicationWindow::from_instance(&win).stack;
                stack.set_visible_child_name("add_password");
            })
        );

        action!(
            window,
            "show-search",
            clone!(@weak window as win => move |_, _| {
                let search_bar = &imp::ApplicationWindow::from_instance(&win).search_bar;
                let search_entry = &imp::ApplicationWindow::from_instance(&win).search_entry;

                // Must be easier to bind those ...
                search_bar.set_search_mode(!search_bar.get_search_mode());
                search_entry.grab_focus();
            })
        );

        action!(
            window,
            "copy",
            clone!(@weak window as win => move |_, _| {
                let password = &imp::ApplicationWindow::from_instance(&win).password;
                let clipboard = password.get_clipboard();
                clipboard.set_text(&password.get_text());

                let info_label = &imp::ApplicationWindow::from_instance(&win).info_label;
                let entry_label = &imp::ApplicationWindow::from_instance(&win).entry_label;
                info_label.set_markup(&format!("Copied <b>{}</b> to clipboard.", entry_label.get_text()));

                let info_bar = &imp::ApplicationWindow::from_instance(&win).info_bar;
                info_bar.set_revealed(true);

                let sender = sender.clone();

                glib::source::timeout_add_seconds(2, move || {
                    let _ = sender.send(Message::HideInfoBar);
                    glib::Continue(false)
                });
            })
        );

        window
    }

    pub fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let settings = &imp::ApplicationWindow::from_instance(self).settings;

        let (width, height) = self.get_default_size();

        settings.set_int("window-width", width)?;
        settings.set_int("window-height", height)?;

        settings.set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_window_size(&self) {
        let settings = &imp::ApplicationWindow::from_instance(self).settings;

        let width = settings.get_int("window-width");
        let height = settings.get_int("window-height");
        let is_maximized = settings.get_boolean("is-maximized");

        self.set_default_size(width, height);

        if is_maximized {
            self.maximize();
        }
    }
}
