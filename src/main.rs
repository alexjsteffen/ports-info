use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Button, CssProvider, Entry, HeaderBar, Label,
    ListBox, MenuButton, Orientation, PolicyType, ScrolledWindow, SearchBar, SearchEntry,
    SelectionMode, Separator, StyleContext, ToggleButton, Widget, WrapMode, STYLE_PROVIDER_PRIORITY_APPLICATION,
    License, MessageDialog,
};
use gtk4::{gdk, gio, glib};
use libadwaita::{self as adw, prelude::*, AboutWindow, Banner, ExpanderRow};
use sysinfo::{Pid, Process, ProcessExt, System, SystemExt};
use glib::Continue;
use pango;
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;

struct PortData {
    port: String,
    pid: Option<u32>,
    name: String,
    protocol: String,
    local_ip: String,
    foreign_address: String,
    state: String,
    recv_q: String,
    send_q: String,
}

struct PortMonitorWindow {
    window: ApplicationWindow,
    all_ports: RefCell<Vec<PortData>>,
    list_box: ListBox,
    search_entry: SearchEntry,
    search_bar: SearchBar,  // Add this field
    warning_banner: Banner,
    is_root: RefCell<bool>,
}

impl PortMonitorWindow {
    fn new(app: &Application) -> Rc<Self> {
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(800)
            .default_height(600)
            .title("Ports Info - Listening Ports Information")
            .build();

        // Main layout
        let main_box = Box::new(Orientation::Vertical, 0);
        window.set_child(Some(&main_box));

        // Header bar
        let header = HeaderBar::new();
        main_box.append(&header);

        // Search button
        let search_button = ToggleButton::builder()
            .icon_name("system-search-symbolic")
            .tooltip_text("Search ports (Ctrl+F)")
            .build();
        header.pack_end(&search_button);

        // Refresh button
        let refresh_button = Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Refresh port information")
            .build();
        header.pack_start(&refresh_button);

        // Menu button
        let menu_button = MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .tooltip_text("Main menu")
            .build();
        let menu = gio::Menu::new();
        menu.append(Some("About"), Some("app.about"));
        menu_button.set_menu_model(Some(&menu));
        header.pack_end(&menu_button);

        // Warning banner
        let warning_banner = Banner::builder()
            .title("Limited port information: Running without administrative privileges")
            .build();
        warning_banner.add_css_class("error");
        warning_banner.set_revealed(false);
        main_box.append(&warning_banner);

        // Search bar
        let search_bar = SearchBar::new();
        let search_entry = SearchEntry::new();
        search_bar.set_key_capture_widget(Some(&window));
        search_bar.connect_entry(&search_entry);
        search_bar.set_child(Some(&search_entry));
        main_box.append(&search_bar);

        // Scrolled window and list box
        let scrolled = ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .build();
        main_box.append(&scrolled);

        let list_box = ListBox::new();
        list_box.set_selection_mode(SelectionMode::None);
        scrolled.set_child(Some(&list_box));

        // CSS styling
        let provider = CssProvider::new();
        provider.load_from_data(
            "
            .error {
                background-color: #f44336;
                color: white;
            }
            .dark {
                background-color: #303030;
                border-radius: 6px;
                padding: 6px;
            }
            .white {
                color: white;
            }
            row {
                padding: 6px;
            }
            .title {
                font-weight: bold;
            }
            .port-number {
                color: #729fcf;
                font-weight: bold;
                font-size: 1.2em;
            }
            ",
        );
        StyleContext::add_provider_for_display(
            &gdk::Display::default().unwrap(),
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let self_ref = Rc::new(Self {
            window,
            all_ports: RefCell::new(Vec::new()),
            list_box,
            search_entry,
            search_bar,  // Add this field
            warning_banner,
            is_root: RefCell::new(false),
        });

        // Connect signals
        let self_clone = Rc::clone(&self_ref);
        search_button.connect_toggled(move |button| {
            self_clone.on_search_toggled(button);
        });

        let self_clone = Rc::clone(&self_ref);
        refresh_button.connect_clicked(move |_| {
            self_clone.refresh_data();
        });

        let self_clone = Rc::clone(&self_ref);
        self_ref
            .search_entry
            .connect_search_changed(move |_| self_clone.on_search_changed());

        // Load port data
        let self_clone = Rc::clone(&self_ref);
        glib::idle_add_local(move || {
            self_clone.load_privileged_data();
            Continue(false)
        });

        self_ref
    }

    fn load_privileged_data(&self) {
        let output = Command::new("pkexec")
            .arg("netstat")
            .arg("-plntu")
            .output();

        match output {
            Ok(output) if output.status.success() => {
                self.is_root.replace(true);
                self.warning_banner.set_revealed(false);
                let stdout = String::from_utf8_lossy(&output.stdout);
                self.parse_netstat_output(&stdout, true);
            }
            _ => {
                self.fallback_to_unprivileged();
            }
        }
    }

    fn fallback_to_unprivileged(&self) {
        self.is_root.replace(false);
        self.warning_banner.set_revealed(true);

        // Try ss command first
        if let Ok(output) = Command::new("ss").args(["-tuan"]).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                self.parse_ss_output(&stdout);
                return;
            }
        }

        // Fallback to unprivileged netstat
        if let Ok(output) = Command::new("netstat").args(["-tun"]).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                self.parse_netstat_output(&stdout, false);
                return;
            }
        }

        // If both fail, show error
        self.show_error_dialog("Failed to get port information. Neither ss nor netstat commands are available.");
    }

    fn refresh_data(&self) {
        if *self.is_root.borrow() {
            self.load_privileged_data();
        } else {
            self.fallback_to_unprivileged();
        }
    }

    fn on_search_toggled(&self, button: &ToggleButton) {
        let active = button.is_active();
        self.search_bar.set_search_mode(active);
    }

    fn on_search_changed(&self) {
        self.list_box.invalidate_filter();
    }

    fn filter_ports(&self, row: &gtk4::ListBoxRow) -> bool {
        let search_text = self.search_entry.text().to_lowercase();
        if search_text.is_empty() {
            return true;
        }

        if let Some(child) = row.child() {
            if let Some(expander_row) = child.downcast_ref::<adw::ExpanderRow>() {
                let title = expander_row.title().to_lowercase();
                let subtitle = expander_row.subtitle().to_lowercase();
                return title.contains(&search_text) || subtitle.contains(&search_text);
            }
        }
        true
    }

    fn show_error_dialog(&self, message: &str) {
        let dialog = MessageDialog::builder()
            .transient_for(&self.window)
            .heading("Error")
            .body(message)
            .build();
        dialog.add_response("ok", "_OK");
        dialog.present();
    }

    fn create_port_row(&self, port_data: &PortData) -> Widget {
        let row = ExpanderRow::new();

        // Set title and subtitle
        let title = format!(
            "<span size='large'>{}</span> <span weight='bold' size='large' color='#729fcf'>{}</span>",
            port_data.protocol.to_uppercase(),
            port_data.port
        );
        row.set_title(&title);

        let subtitle = if let Some(pid) = port_data.pid {
            format!("{} (PID: {})", port_data.name, pid)
        } else {
            port_data.name.clone()
        };
        row.set_subtitle(&subtitle);

        // Details box
        let details_box = Box::new(Orientation::Vertical, 6);
        details_box.set_margin_start(12);
        details_box.set_margin_end(12);
        details_box.set_margin_top(6);
        details_box.set_margin_bottom(6);
        details_box.add_css_class("dark");

        // Helper to create labels
        let create_detail_label = |text: &str| {
            let label = Label::builder()
                .label(text)
                .xalign(0.0)
                .build();
            label.set_wrap(true);
            label.set_wrap_mode(WrapMode::WordChar);
            label.set_hexpand(true);
            label.add_css_class("white");
            label
        };

        // Add details
        details_box.append(&create_detail_label(&format!("Protocol: {}", port_data.protocol.to_uppercase())));
        details_box.append(&create_detail_label(&format!("Local Address: {}:{}", port_data.local_ip, port_data.port)));
        details_box.append(&create_detail_label(&format!("Foreign Address: {}", port_data.foreign_address)));
        details_box.append(&create_detail_label(&format!("State: {}", port_data.state)));

        // Process details (if available)
        if let Some(pid) = port_data.pid {
            if let Some(process_info) = self.get_process_info(pid) {
                // Separator
                let separator = Separator::new(Orientation::Horizontal);
                separator.add_css_class("white");
                separator.set_margin_top(6);
                separator.set_margin_bottom(6);
                details_box.append(&separator);

                // Command
                let cmdline = process_info.cmd().join(" ");
                if !cmdline.is_empty() {
                    details_box.append(&create_detail_label(&format!("Command: {}", cmdline)));
                }
                // User
                if let Some(username) = process_info.user_name() {
                    details_box.append(&create_detail_label(&format!("User: {}", username)));
                }
                // CPU Usage
                details_box.append(&create_detail_label(&format!("CPU Usage: {:.1}%", process_info.cpu_usage())));
                // Memory Usage
                details_box.append(&create_detail_label(&format!("Memory Usage: {:.1} MB", process_info.memory() as f64 / 1024.0 / 1024.0)));
                // Start Time
                if let Some(start_time) = Self::format_start_time(process_info.start_time() as i64) {
                    details_box.append(&create_detail_label(&format!("Started: {}", start_time)));
                }
                // Status
                details_box.append(&create_detail_label(&format!("Status: {:?}", process_info.status())));
            }
        }

        // Scrolled window for details
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Never)
            .child(&details_box)
            .build();

        row.add_row(&scrolled);
        row.upcast::<Widget>()
    }

    fn parse_netstat_output(&self, output: &str, privileged: bool) {
        let mut ports = Vec::new();
        let mut system = System::new_all();

        if privileged {
            system.refresh_all();
        }

        for line in output.lines().skip(2) {
            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let protocol = parts[0].to_lowercase();
                let local_address = parts[3];
                let foreign_address = parts[4];
                let (state, pid_info) = if protocol.starts_with("tcp") {
                    (parts.get(5).unwrap_or(&"unknown").to_string(), parts.get(6))
                } else {
                    ("stateless".to_string(), parts.get(5))
                };

                let (pid, name) = if privileged {
                    if let Some(&pid_prog) = pid_info {
                        if pid_prog != "-" {
                            if let Some((pid_str, prog_name)) = pid_prog.split_once('/') {
                                (pid_str.parse::<u32>().ok(), prog_name.to_string())
                            } else {
                                (pid_prog.parse::<u32>().ok(), "Unknown".to_string())
                            }
                        } else {
                            (None, "Unknown".to_string())
                        }
                    } else {
                        (None, "Unknown".to_string())
                    }
                } else {
                    (None, "Unknown (no privileges)".to_string())
                };

                let port = local_address.rsplitn(2, ':').next().unwrap_or("").to_string();
                let local_ip = local_address.rsplitn(2, ':').nth(1).unwrap_or("Any").to_string();

                let port_data = PortData {
                    port,
                    pid,
                    name,
                    protocol,
                    local_ip,
                    foreign_address: foreign_address.to_string(),
                    state,
                    recv_q: "0".to_string(),
                    send_q: "0".to_string(),
                };
                ports.push(port_data);
            }
        }
        self.all_ports.replace(ports);
        self.refresh_display();
    }

    fn parse_ss_output(&self, output: &str) {
        let mut ports = Vec::new();

        for line in output.lines().skip(1) {
            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                let protocol = parts[0].to_lowercase();
                let state = if protocol.starts_with("tcp") { parts[1] } else { "stateless" }.to_string();

                let local_address = parts[4];
                let foreign_address = parts.get(5).unwrap_or(&"*:*");

                let port = local_address.rsplitn(2, ':').next().unwrap_or("").to_string();
                let local_ip = local_address.rsplitn(2, ':').nth(1).unwrap_or("Any").to_string();

                let port_data = PortData {
                    port,
                    pid: None,
                    name: "Unknown (no privileges)".to_string(),
                    protocol,
                    local_ip,
                    foreign_address: foreign_address.to_string(),
                    state,
                    recv_q: "0".to_string(),
                    send_q: "0".to_string(),
                };
                ports.push(port_data);
            }
        }
        self.all_ports.replace(ports);
        self.refresh_display();
    }

    fn refresh_display(&self) {
        let children: Vec<_> = self.list_box.children().collect();
        for child in children {
            self.list_box.remove(&child);
        }
        for port_data in self.all_ports.borrow().iter() {
            let row = self.create_port_row(port_data);
            self.list_box.append(&row);
        }
    }

    fn get_process_info(&self, pid: u32) -> Option<Process> {
        let mut system = System::new_all();
        system.refresh_process(Pid::from(pid));
        system.process(Pid::from(pid)).cloned()
    }

    // Fix the DateTime handling
    fn format_start_time(timestamp: i64) -> Option<String> {
        use chrono::{DateTime, Local, TimeZone};
        Local.timestamp_opt(timestamp, 0)
            .single()
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
    }
}

fn main() {
    let app = Application::builder()
        .application_id("com.github.mfat.ports-info")
        .build();

    app.connect_activate(|app| {
        let port_monitor_window = PortMonitorWindow::new(app);
        port_monitor_window.window.present();
    });

    // Set up "about" action
    let about_action = gio::SimpleAction::new("about", None);
    about_action.connect_activate(|_, _| {
        let about = AboutWindow::builder()
            .application_name("PortsInfo")
            .application_icon("security-medium")
            .developer_name("mFat")
            .version("1.0")
            .website("https://github.com/mfat/ports")
            .license_type(License::Gpl30)
            .developers(vec!["mFat".to_string()])
            .copyright("2024 mFat")
            .build();
        about.show();
    });
    app.add_action(&about_action);

    app.run();
}