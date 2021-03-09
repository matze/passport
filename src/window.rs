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
use std::rc::Rc;

mod imp {
    use super::*;

    #[derive(CompositeTemplate)]
    #[template(resource = "/net/bloerg/Passport/window.ui")]
    pub struct ApplicationWindow {
        pub storage: Rc<storage::Storage>,
        pub settings: gio::Settings,
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
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ApplicationWindow {
        const NAME: &'static str = "ApplicationWindow";
        type Type = super::ApplicationWindow;
        type ParentType = gtk::ApplicationWindow;

        fn new() -> Self {
            Self {
                storage: Rc::new(storage::Storage::new().unwrap()),
                settings: gio::Settings::new(APP_ID),
                search_bar: TemplateChild::default(),
                search_entry: TemplateChild::default(),
                store: TemplateChild::default(),
                selection: TemplateChild::default(),
                password: TemplateChild::default(),
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

            for entry in &self.storage.entries {
                // We re-use GtkLabel which is lame but I am too stupid to define my own simple
                // GObject subclass.
                let label = gtk::Label::new(Some(&self.storage.entry_name(&entry).unwrap()));
                label.set_halign(gtk::Align::Start);
                self.store.append(&label);
            }

            self.selection.connect_selection_changed(
                clone!(@strong self.storage as storage, @strong self.password as password => move |selection, _, _| {
                    if let Some(item) = selection.get_object(selection.get_selected()) {
                        let label = item.downcast::<gtk::Label>().unwrap();
                        // make this async
                        let entry = storage.decrypt(label.get_text().as_str()).unwrap();
                        password.set_text(&entry.password);
                    }
                }),
            );
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
                let selection = &imp::ApplicationWindow::from_instance(&win).selection;

                if let Some(item) = selection.get_object(selection.get_selected()) {
                    let storage = &imp::ApplicationWindow::from_instance(&win).storage;
                    let label = item.downcast::<gtk::Label>().unwrap();
                    let entry = storage.decrypt(label.get_text().as_str()).unwrap();
                    let clipboard = label.get_clipboard();
                    clipboard.set_text(&entry.password);
                }
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
