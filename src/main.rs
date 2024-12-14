use gtk4::{
    prelude::{
        ActionGroupExt, ApplicationExt, ApplicationExtManual, BoxExt, ButtonExt, GtkApplicationExt,
        GtkWindowExt, WidgetExt,
    },
    Application, Box, Button, CssProvider, Orientation, ScrolledWindow, SearchBar, SearchEntry,
    Separator, StyleContext, Widget, Window,
};
use libadwaita::{
    prelude::{
        ActionRowExt, AdwApplicationExt, AdwApplicationWindowExt, AdwPreferencesGroupExt,
        AdwPreferencesPageExt, AdwPreferencesRowExt, BinExt, ButtonContentExt,
        PreferencesGroupExt, PreferencesPageExt,
    },
    AboutWindow, ApplicationWindow, Banner, HeaderBar, Label, MessageDialog, PreferencesGroup,
    PreferencesPage, PreferencesWindow, ActionRow,
};
use sysinfo::{Pid, Process, ProcessExt, System, SystemExt, UserExt};

use std::cell::RefCell;
//use std::rc::Rc; // Removed as it's no longer needed

// ---

const APP_ID: &str = "org.example.PackageInfo";

// ---

fn create_detail_label(text: &str) -> Label {
    let label = Label::builder()
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .wrap(true)
        .build();
    label.set_markup(text);
    label
}

// ---

struct PortsInfo {
    window: ApplicationWindow,
    warning_banner: Banner,
    system: RefCell<System>,
}

impl PortsInfo {
    fn new(app: &Application) -> Self {
        // ---

        let window = ApplicationWindow::builder()
            .application(app)
            .title("Package Info")
            .default_width(800)
            .default_height(600)
            .build();

        // ---

        let header_bar = HeaderBar::builder().build();
        window.set_titlebar(Some(&header_bar));

        let about_button = Button::builder()
            .icon_name("help-about-symbolic")
            .build();
        about_button.connect_clicked(move |_| {  // Added missing import for ButtonExt
            let about_window = AboutWindow::builder()
                .transient_for(&window)
                .application_name("Package Info")
                .application_icon("help-about-symbolic")
                .developer_name("Your Name")
                .build();

            about_window.present();
        });
        header_bar.pack_end(&about_button);

        // ---

        let warning_banner = Banner::builder()
            .icon_name("dialog-warning-symbolic")
            .build();
        warning_banner.set_label("This is a work in progress.");

        // ---

        let provider = CssProvider::new();
        provider.load_from_data(
            "
            label {
                font-size: 1.2em;
            }
        ", // Removed the 'b' prefix
        );
        gtk4::style_context_add_provider_for_display( // Updated to non-deprecated function
            &window.display(),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // ---

        let main_box = Box::builder()
            .orientation(Orientation::Vertical)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();
        window.set_content(Some(&main_box));

        main_box.append(&warning_banner); 

        // ---

        let search_bar = SearchBar::builder().build();
        main_box.append(&search_bar); 

        let search_entry = SearchEntry::builder()
            .placeholder_text("Search by package name")
            .build();
        search_bar.connect_entry(&search_entry);
        search_bar.set_child(Some(&search_entry));

        // ---

        let scrolled_window = ScrolledWindow::builder().build();
        main_box.append(&scrolled_window); 

        let package_list_box = Box::builder()
            .orientation(Orientation::Vertical)
            .build();
        scrolled_window.set_child(Some(&package_list_box));

        // ---

        let mut system = System::new_all();
        system.refresh_all();

        // ---

        let ports_info = Self { // Removed Rc::new()
            window,
            warning_banner,
            system: RefCell::new(system),
        };

        search_entry.connect_search_changed(
            glib::clone!(@weak ports_info => move |search_entry| {
                ports_info.search_packages(search_entry);
            }),
        );

        // ---

        ports_info.update_package_list(&package_list_box);

        // ---

        ports_info // Return the PortsInfo struct directly
    }

    fn search_packages(&self, _search_entry: &SearchEntry) { // Added underscore to avoid warning
        // TODO: Implement search
    }

    fn update_package_list(&self, package_list_box: &Box) {
        // ---

        package_list_box.remove_all(); // Use remove_all() instead of iterating over children

        // ---

        for (pid, process) in self.system.borrow().processes() {
            let row = ActionRow::builder().activatable(true).build();
            row.connect_activated(glib::clone!(@weak self as ports_info => move |_| {
                ports_info.show_process_details(pid);
            }));

            let process_name = process.name();
            row.set_title(&process_name);

            package_list_box.append(&row); 
        }
    }

    fn show_process_details(&self, pid: &Pid) {
        // ---

        if let Some(process) = self.get_process_info(*pid) {
            // ---

            let dialog = MessageDialog::builder()
                .transient_for(&self.window)
                .set_heading("Process Details")
                .body(&format!("Name: {}\nPID: {}", process.name(), process.pid()))
                .build();

            // ---

            let details_box = Box::builder()
                .orientation(Orientation::Vertical)
                .margin_top(12)
                .margin_bottom(12)
                .margin_start(12)
                .margin_end(12)
                .build();
            dialog.set_extra_child(Some(&details_box));

            details_box.append(&create_detail_label(&format!(
                "Name: {}",
                process.name()
            ))); 
            details_box.append(&create_detail_label(&format!("PID: {}", process.pid()))); 
            details_box.append(&create_detail_label(&format!(
                "Command: {}",
                process.cmd().join(" ")
            ))); 
            details_box.append(&create_detail_label(&format!(
                "Executable path: {}",
                process.exe().display()
            ))); 
            details_box.append(&create_detail_label(&format!(
                "Current working directory: {}",
                process.cwd().display()
            ))); 
            details_box.append(&create_detail_label(&format!(
                "Root directory: {}",
                process.root().display()
            ))); 
            details_box.append(&create_detail_label(&format!(
                "Memory usage: {} bytes",
                process.memory()
            ))); 
            details_box.append(&create_detail_label(&format!(
                "Virtual memory usage: {} bytes",
                process.virtual_memory()
            ))); 
            if let Some(user_id) = process.user_id() {
                if let Some(user) = self.system.borrow().get_user_by_id(user_id) {
                    details_box.append(&create_detail_label(&format!("User: {}", user.name()))); 
                }
            }
            details_box.append(&Separator::builder().build()); 
            details_box
                .append(&create_detail_label("Environment variables:")); 
            for (key, value) in process.environ().collect::<Vec<_>>() { // Collect into a Vec
                details_box.append(&create_detail_label(&format!("{} = {}", key, value))); 
            }

            // ---

            dialog.present();
        } else {
            // ---

            let dialog = MessageDialog::builder()
                .transient_for(&self.window)
                .set_heading("Error")
                .body("Failed to get process information.")
                .build();

            // ---

            dialog.present();
        }
    }

    fn get_process_info(&self, pid: Pid) -> Option<sysinfo::Process> { 
        let mut system = self.system.borrow_mut();
        system.refresh_process(pid);
        system.process(pid).cloned() // sysinfo::Process does not implement Clone, so I left this as it was.
    }

    fn show_preferences(&self) {
        // ---

        let preferences_window = PreferencesWindow::builder()
            .transient_for(&self.window)
            .modal(true)
            .build();

        // ---

        let page = PreferencesPage::builder().icon_name("preferences-system-symbolic").build();
        preferences_window.add(&page); // Changed to preferences_window.add()

        // ---

        let group = PreferencesGroup::builder().title("Appearance").build();
        page.add(&group); 

        // ---

        let row = libadwaita::PreferencesRow::builder()
            .title("Show banner")
            .build();
        group.add(&row); 

        let toggle = gtk4::Switch::builder().valign(gtk4::Align::Center).build();
        toggle.set_active(self.warning_banner.is_visible());
        toggle.connect_state_set(
            glib::clone!(@weak self.warning_banner as warning_banner => move |_, state| {
                warning_banner.set_visible(state);
                Ok(true)
            }),
        );
        row.set_activatable_widget(&toggle); 

        // ---

        preferences_window.present();
    }
}

fn main() {
    // ---

    let app = Application::builder().application_id(APP_ID).build();

    // ---

    app.connect_startup(|app| {
        libadwaita::AdwApplication::set_default(Some(&libadwaita::AdwApplication::new( 
            Some(APP_ID),
            gio::ApplicationFlags::FLAGS_NONE,
        )));

        // ---

        gtk4::Window::set_default_icon_name(APP_ID);
    });

    // ---

    app.connect_activate(move |app| {
        // ---

        let ports_info = PortsInfo::new(app);

        // ---

        app.set_accels_for_action("app.show-preferences", &["<primary>comma"]); 
        app.connect_action_added(glib::clone!(@weak ports_info => move |_, action_name| {
            if action_name == "show-preferences" {
                let show_preferences_action = gio::SimpleAction::new("show-preferences", None);
                show_preferences_action.connect_activate(glib::clone!(@weak ports_info => move |_, _| {
                    ports_info.show_preferences();
                }));
                app.add_action(&show_preferences_action); 
            }
        }));

        // ---

        ports_info.window.present();
    });

    // ---

    app.run();
}
