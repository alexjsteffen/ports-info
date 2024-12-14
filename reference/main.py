#!/usr/bin/env python3
import gi
gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')
from gi.repository import Gtk, Adw, GLib, Gio, Gdk, Pango
import subprocess
import pwd
import os
import psutil
from datetime import datetime

class PortMonitorWindow(Adw.ApplicationWindow):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.set_default_size(800, 600)
        self.set_title("Ports Info - Listening Ports Information")
        self.all_ports = []
        self.is_root = False

        # Set up search action
        search_action = Gio.SimpleAction.new("search", None)
        search_action.connect("activate", self.toggle_search)
        self.add_action(search_action)

        # Main layout
        self.main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        self.set_content(self.main_box)

        # Header bar first
        header = Adw.HeaderBar()
        self.main_box.append(header)

        # Search button
        self.search_button = Gtk.ToggleButton(icon_name="system-search-symbolic")
        self.search_button.set_tooltip_text("Search ports (Ctrl+F)")
        self.search_button.connect("toggled", self.on_search_toggled)
        header.pack_end(self.search_button)

        # Refresh button
        refresh_button = Gtk.Button(icon_name="view-refresh-symbolic")
        refresh_button.set_tooltip_text("Refresh port information")
        refresh_button.connect("clicked", self.refresh_data)
        header.pack_start(refresh_button)

        # Menu button
        menu_button = Gtk.MenuButton()
        menu_button.set_icon_name("open-menu-symbolic")
        menu_button.set_tooltip_text("Main menu")
        header.pack_end(menu_button)

        # Create menu
        menu = Gio.Menu()
        menu.append("About", "app.about")
        menu_button.set_menu_model(menu)

        # Warning banner after header
        self.warning_banner = Adw.Banner()
        self.warning_banner.set_title("Limited port information: Running without administrative privileges")
        self.warning_banner.add_css_class("error")  # Red warning style
        self.warning_banner.set_revealed(False)
        self.main_box.append(self.warning_banner)

        # Search bar
        self.search_bar = Gtk.SearchBar()
        self.search_entry = Gtk.SearchEntry()
        self.search_entry.set_hexpand(True)
        self.search_entry.connect("search-changed", self.on_search_changed)
        self.search_bar.set_child(self.search_entry)
        self.search_bar.set_key_capture_widget(self)
        self.search_bar.connect_entry(self.search_entry)
        self.main_box.append(self.search_bar)

        # Create scrolled window
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_vexpand(True)
        self.main_box.append(scrolled)

        # Create list box for ports
        self.list_box = Gtk.ListBox()
        self.list_box.set_selection_mode(Gtk.SelectionMode.NONE)
        self.list_box.add_css_class("boxed-list")
        self.list_box.set_filter_func(self.filter_ports)
        scrolled.set_child(self.list_box)

        # Add loading spinner
        self.spinner_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        self.spinner_box.set_valign(Gtk.Align.CENTER)
        self.spinner_box.set_vexpand(True)
        
        self.spinner = Gtk.Spinner()
        self.spinner.set_size_request(32, 32)
        self.spinner_box.append(self.spinner)
        
        loading_label = Gtk.Label(label="Loading port information...")
        self.spinner_box.append(loading_label)
        
        self.list_box.append(self.spinner_box)
        self.spinner.start()

        # Try to load privileged data after window is shown
        GLib.idle_add(self.load_privileged_data)

        # Add CSS provider for custom styles
        css_provider = Gtk.CssProvider()
        css_provider.load_from_data(b"""
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
            row > box > label.title {
                font-weight: bold;
            }
            .port-number {
                color: #729fcf;
                font-weight: bold;
                font-size: 1.2em;
            }
        """)
        
        Gtk.StyleContext.add_provider_for_display(
            Gdk.Display.get_default(),
            css_provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
        )

    def load_privileged_data(self):
        """Load data using netstat with root privileges"""
        try:
            # Clear the spinner
            if self.spinner_box.get_parent():
                self.list_box.remove(self.spinner_box)

            output = subprocess.run(
                ["pkexec", "netstat", "-plntu"],
                capture_output=True,
                text=True,
                check=True
            ).stdout
            
            self.is_root = True
            self.warning_banner.set_revealed(False)
            self.parse_netstat_output(output, privileged=True)
            
        except subprocess.CalledProcessError as e:
            print(f"Authentication failed or cancelled: {e}")
            self.fallback_to_unprivileged()
        except Exception as e:
            print(f"Error during privileged access: {e}")
            self.fallback_to_unprivileged()

    def fallback_to_unprivileged(self):
        """Fallback to unprivileged mode"""
        self.is_root = False
        self.warning_banner.set_revealed(True)
        
        try:
            # Clear the spinner if it's still there
            if self.spinner_box.get_parent():
                self.list_box.remove(self.spinner_box)

            # Try ss command first
            output = subprocess.check_output(
                ["ss", "-tuan"],
                stderr=subprocess.PIPE,
                text=True
            )
            self.parse_ss_output(output)
        except (subprocess.CalledProcessError, FileNotFoundError):
            try:
                # Fallback to unprivileged netstat if ss fails
                output = subprocess.check_output(
                    ["netstat", "-tun"],
                    stderr=subprocess.PIPE,
                    text=True
                )
                self.parse_netstat_output(output, privileged=False)
            except subprocess.CalledProcessError as e:
                print(f"Both ss and netstat failed: {e}")
                self.show_error_dialog("Failed to get port information. Neither ss nor netstat commands are available.")

    def refresh_data(self, *args):
        """Refresh the port data"""
        if self.is_root:
            self.load_privileged_data()
        else:
            self.fallback_to_unprivileged()

    def on_search_toggled(self, button):
        self.search_bar.set_search_mode(button.get_active())

    def on_search_changed(self, entry):
        self.list_box.invalidate_filter()

    def filter_ports(self, row):
        if not self.search_entry.get_text():
            return True

        search_text = self.search_entry.get_text().lower()
        if hasattr(row, 'get_title'):  # Check if it's our ExpanderRow
            title = row.get_title().lower()
            subtitle = row.get_subtitle().lower()
            return search_text in title or search_text in subtitle
        return True

    def show_error_dialog(self, message):
        dialog = Adw.MessageDialog.new(
            transient_for=self,
            heading="Error",
            body=message
        )
        dialog.add_response("ok", "_OK")
        dialog.present()

    def run_with_sudo(self, cmd):
        if os.geteuid() == 0:
            try:
                return subprocess.check_output(cmd, stderr=subprocess.PIPE).decode()
            except subprocess.CalledProcessError as e:
                self.show_error_dialog(f"Failed to execute command: {e}")
                return None

        # Use pkexec for elevated privileges
        try:
            pkexec_cmd = ["pkexec"] + cmd
            return subprocess.check_output(pkexec_cmd, stderr=subprocess.PIPE).decode()
        except (subprocess.CalledProcessError, FileNotFoundError) as e:
            self.show_error_dialog(f"Authentication failed: {e}")
            return None

    def get_port_data(self):
        try:
            if self.is_root:
                # Try privileged netstat
                output = self.run_with_sudo(["netstat", "-plntu"])
                if output is None:
                    raise Exception("Failed to run privileged netstat")
            else:
                # Fallback to unprivileged ss command
                try:
                    output = subprocess.check_output(
                        ["ss", "-tuan"],
                        stderr=subprocess.PIPE
                    ).decode()
                except subprocess.CalledProcessError:
                    # If ss fails, try unprivileged netstat
                    output = subprocess.check_output(
                        ["netstat", "-tun"],
                        stderr=subprocess.PIPE
                    ).decode()

            print("Raw netstat output:", output)  # Debug print
                
            ports = []
            for line in output.split('\n')[2:]:  # Skip header lines
                if not line:
                    continue
                    
                parts = line.split()
                print("Line parts:", parts)  # Debug print
                
                if len(parts) >= 7:
                    protocol = parts[0].lower()
                    recv_q = parts[1]
                    send_q = parts[2]
                    local_address = parts[3]
                    foreign_address = parts[4]
                    state = parts[5] if len(parts) > 5 and protocol != "udp" else "stateless"
                    
                    # Handle PID/Program name field
                    pid = None
                    name = 'Unknown'
                    
                    if len(parts) > 6:
                        pid_info = parts[-1]  # Get the last field which should be PID/Program
                        print("PID info:", pid_info)  # Debug print
                        
                        if pid_info != '-':
                            if '/' in pid_info:
                                pid_str, name = pid_info.split('/', 1)
                            else:
                                pid_str = pid_info
                            
                            try:
                                pid = int(pid_str)
                            except ValueError:
                                pid = None
                    
                    if ":" in local_address:
                        local_ip, port = local_address.rsplit(":", 1)
                        if local_ip == "0.0.0.0" or local_ip == "::":
                            local_ip = "Any"
                            
                        # Format foreign address
                        if ":" in foreign_address:
                            foreign_ip, foreign_port = foreign_address.rsplit(":", 1)
                            if foreign_ip == "0.0.0.0" or foreign_ip == "::" or foreign_ip == "*":
                                foreign_ip = "Any"
                            foreign_address = f"{foreign_ip}:{foreign_port}"
                        
                        port_data = {
                            'port': port,
                            'pid': pid,
                            'name': name,
                            'protocol': protocol,
                            'local_ip': local_ip,
                            'foreign_address': foreign_address,
                            'state': state,
                            'recv_q': recv_q,
                            'send_q': send_q
                        }
                        print("Port data:", port_data)  # Debug print
                        ports.append(port_data)
            
            return ports
        except Exception as e:
            self.show_error_dialog(f"Failed to get port information: {str(e)}")
            return []

    def create_port_row(self, port_data):
        row = Adw.ExpanderRow()
        
        # Create title showing protocol and port with larger text
        title = f"<span font_size='large'>{port_data['protocol'].upper()}</span> <span font_weight='bold' font_size='large' color='#729fcf'>{port_data['port']}</span>"
        row.set_title(title)
        
        # Create subtitle showing PID and program name
        if port_data['pid']:
            subtitle = f"{port_data['name']} (PID: {port_data['pid']})"
        else:
            subtitle = f"{port_data['name']}"
        row.set_subtitle(subtitle)

        # Create details box
        details_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        details_box.set_margin_start(12)
        details_box.set_margin_end(12)
        details_box.set_margin_top(6)
        details_box.set_margin_bottom(6)
        details_box.add_css_class("dark")  # Add dark background
        
        # Helper function to create labels with proper wrapping
        def create_detail_label(text):
            label = Gtk.Label(label=text, xalign=0)
            label.set_wrap(True)
            label.set_wrap_mode(Pango.WrapMode.WORD_CHAR)
            label.set_hexpand(True)
            label.add_css_class("white")  # Add white text
            return label

        # Network Details
        details_box.append(create_detail_label(
            f"Protocol: {port_data['protocol'].upper()}"
        ))
        details_box.append(create_detail_label(
            f"Local Address: {port_data['local_ip']}:{port_data['port']}"
        ))
        details_box.append(create_detail_label(
            f"Foreign Address: {port_data['foreign_address']}"
        ))
        details_box.append(create_detail_label(
            f"State: {port_data['state']}"
        ))

        # Process Details
        if port_data['pid'] and port_data['process_info']:
            try:
                process_info = port_data['process_info']
                
                # Add a separator with white color
                separator = Gtk.Separator(orientation=Gtk.Orientation.HORIZONTAL)
                separator.add_css_class("white")  # Make separator white
                separator.set_margin_top(6)
                separator.set_margin_bottom(6)
                details_box.append(separator)
                
                # Command
                if process_info.get('cmdline'):
                    details_box.append(create_detail_label(
                        f"Command: {' '.join(process_info['cmdline'])}"
                    ))
                
                # User
                if process_info.get('username'):
                    details_box.append(create_detail_label(
                        f"User: {process_info['username']}"
                    ))
                
                # CPU Usage
                if process_info.get('cpu_percent') is not None:
                    details_box.append(create_detail_label(
                        f"CPU Usage: {process_info['cpu_percent']:.1f}%"
                    ))
                
                # Memory Usage
                if process_info.get('memory_info'):
                    memory_mb = process_info['memory_info'].rss / 1024 / 1024
                    details_box.append(create_detail_label(
                        f"Memory Usage: {memory_mb:.1f} MB"
                    ))
                
                # Creation Time
                if process_info.get('create_time'):
                    create_time_str = datetime.fromtimestamp(process_info['create_time']).strftime('%Y-%m-%d %H:%M:%S')
                    details_box.append(create_detail_label(
                        f"Started: {create_time_str}"
                    ))
                
                # Status
                if process_info.get('status'):
                    details_box.append(create_detail_label(
                        f"Status: {process_info['status']}"
                    ))

            except Exception as e:
                details_box.append(create_detail_label(
                    f"Process information unavailable: {str(e)}"
                ))

        # Create a scrolled window to contain the details box
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.NEVER)
        scrolled.set_child(details_box)

        row.add_row(scrolled)
        return row

    def parse_ss_output(self, output):
        """Parse ss output with limited information"""
        try:
            ports = []
            for line in output.split('\n')[1:]:  # Skip header line
                if not line.strip():
                    continue
                
                parts = line.split()
                if len(parts) >= 5:
                    protocol = parts[0].lower()
                    if protocol.startswith('tcp') or protocol.startswith('udp'):
                        state = parts[1] if protocol.startswith('tcp') else "stateless"
                        local_address = parts[4]
                        foreign_address = parts[5] if len(parts) > 5 else "*:*"
                        
                        if ":" in local_address:
                            local_ip, port = local_address.rsplit(":", 1)
                            if local_ip == "*" or local_ip == "0.0.0.0" or local_ip == "::":
                                local_ip = "Any"
                            
                            if ":" in foreign_address:
                                foreign_ip, foreign_port = foreign_address.rsplit(":", 1)
                                if foreign_ip == "*" or foreign_ip == "0.0.0.0" or foreign_ip == "::":
                                    foreign_ip = "Any"
                                foreign_address = f"{foreign_ip}:{foreign_port}"
                            
                            ports.append({
                                'port': port,
                                'pid': None,
                                'name': 'Unknown (no privileges)',
                                'protocol': protocol,
                                'local_ip': local_ip,
                                'foreign_address': foreign_address,
                                'state': state,
                                'recv_q': '0',
                                'send_q': '0'
                            })
            
            self.all_ports = ports
            self.refresh_display()
        except Exception as e:
            print(f"Error parsing ss output: {e}")
            self.show_error_dialog(f"Failed to parse port information: {str(e)}")

    def parse_netstat_output(self, output, privileged=True):
        """Parse netstat output"""
        try:
            ports = []
            processes = {}  # Cache for process info
            
            # Pre-fetch process information if privileged
            if privileged:
                for proc in psutil.process_iter(['pid', 'name', 'username', 'cmdline', 'cpu_percent', 'memory_info', 'create_time', 'status']):
                    try:
                        processes[proc.pid] = proc.info
                    except (psutil.NoSuchProcess, psutil.AccessDenied, psutil.ZombieProcess):
                        continue

            for line in output.split('\n')[2:]:  # Skip header lines
                if not line.strip():
                    continue
                    
                parts = line.split()
                if len(parts) >= 4:  # Minimum fields needed
                    protocol = parts[0].lower()
                    local_address = parts[3]
                    foreign_address = parts[4] if len(parts) > 4 else "*:*"
                    
                    # Handle state differently for UDP and TCP
                    if protocol.startswith('tcp'):
                        state = parts[5] if len(parts) > 5 else "unknown"
                        pid_index = 6
                    else:  # UDP
                        state = "stateless"
                        pid_index = 5  # UDP lines have one less column (no state)
                    
                    # Handle PID/Program name field if available (privileged mode)
                    pid = None
                    name = 'Unknown (no privileges)'
                    process_info = None
                    
                    if privileged and len(parts) > pid_index:
                        pid_info = parts[pid_index]
                        if pid_info != '-':
                            if '/' in pid_info:
                                pid_str, name = pid_info.split('/', 1)
                                try:
                                    pid = int(pid_str)
                                    process_info = processes.get(pid)
                                except ValueError:
                                    pid = None
                    
                    if ":" in local_address:
                        local_ip, port = local_address.rsplit(":", 1)
                        if local_ip == "0.0.0.0" or local_ip == "::" or local_ip == "*":
                            local_ip = "Any"
                            
                        ports.append({
                            'port': port,
                            'pid': pid,
                            'name': name,
                            'protocol': protocol,
                            'local_ip': local_ip,
                            'foreign_address': foreign_address,
                            'state': state,
                            'recv_q': '0',
                            'send_q': '0',
                            'process_info': process_info  # Add cached process info
                        })
            
            print(f"Parsed {len(ports)} ports ({sum(1 for p in ports if p['protocol'].startswith('udp'))} UDP)")
            self.all_ports = ports
            self.refresh_display()
        except Exception as e:
            print(f"Error parsing netstat output: {e}")
            self.show_error_dialog(f"Failed to parse port information: {str(e)}")

    def refresh_display(self):
        """Update the display with the current port data"""
        # Clear existing rows
        while True:
            row = self.list_box.get_first_child()
            if row is None:
                break
            self.list_box.remove(row)

        # Add new rows
        for port_data in self.all_ports:
            row = self.create_port_row(port_data)
            self.list_box.append(row)

    def toggle_search(self, action, param):
        """Toggle search when Ctrl+F is pressed"""
        self.search_button.set_active(not self.search_button.get_active())

class PortMonitorApp(Adw.Application):
    def __init__(self):
        super().__init__(application_id="com.github.mfat.ports-info",
                        flags=Gio.ApplicationFlags.FLAGS_NONE)
        self.connect('activate', self.on_activate)
        self.connect('shutdown', self.on_shutdown)  # Add shutdown handler
        
        # Set default app icon
        icon_theme = Gtk.IconTheme.get_for_display(Gdk.Display.get_default())
        icon_theme.add_search_path("/usr/share/icons/hicolor/scalable/apps")
        
        # Add keyboard shortcuts
        self.set_accels_for_action("win.search", ["<Control>f"])
        
        about_action = Gio.SimpleAction.new("about", None)
        about_action.connect("activate", self.on_about_action)
        self.add_action(about_action)

    def on_activate(self, app):
        win = PortMonitorWindow(application=app)
        win.set_icon_name("ports-info")
        win.present()

    def on_shutdown(self, app):
        # Ensure clean shutdown
        for window in self.get_windows():
            window.is_shutting_down = True
            window.close()

    def on_about_action(self, action, param):
        about = Adw.AboutWindow(
            transient_for=self.get_active_window(),
            application_name="PortsInfo",
            application_icon="security-medium",
            developer_name="mFat",
            version="1.0",
            website="https://github.com/mfat/ports",
            license_type=Gtk.License.GPL_3_0,
            developers=["mFat"],
            copyright=" 2024 mFat"
        )
        about.present()

app = PortMonitorApp()
app.run(None)