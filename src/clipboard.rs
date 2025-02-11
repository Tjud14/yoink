// src/clipboard.rs
use std::process::{Command, Stdio};
use std::io::Write;
use std::thread;
use std::time::Duration;

pub struct ClipboardManager {
    verbose: bool,
}

impl ClipboardManager {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    pub fn copy_to_clipboard(&self, text: &str) -> Result<(), String> {
        // Try Wayland methods first if we're in a Wayland session
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            let wayland_methods = [
                (vec!["wl-copy"], "wl-copy"),
                (vec!["wl-clipboard"], "wl-clipboard"),
            ];
            if self.try_methods(&wayland_methods, text)? {
                return Ok(());
            }
        }

        // Try X11 methods if we're in an X session
        if std::env::var("DISPLAY").is_ok() {
            let x11_methods = [
                (vec!["xclip", "-selection", "clipboard"], "xclip"),
                (vec!["xsel", "-i", "-b"], "xsel"),
            ];
            if self.try_methods(&x11_methods, text)? {
                return Ok(());
            }
        }

        // Get desktop environment and try DE-specific methods
        let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
        let desktop_lower = desktop.to_lowercase();
        
        match desktop_lower.as_str() {
            "x-cinnamon" | "cinnamon" => {
                let formatted_text = format!("string:{}", text);
                let cinnamon_methods = [
                    (vec!["dbus-send", "--session", "--type=method_call", 
                         "--dest=org.Cinnamon", "/org/Cinnamon", 
                         "org.Cinnamon.SetClipboardText", 
                         &formatted_text], "Cinnamon DBus"),
                ];
                if self.try_methods(&cinnamon_methods, text)? {
                    return Ok(());
                }
            },
            "kde" | "plasma" | "plasma5" => {
                let kde_methods = [
                    (vec!["qdbus", "org.kde.klipper", "/klipper", "setClipboardContents", text], "KDE DBus"),
                    (vec!["klipper", "-e"], "Klipper"),
                ];
                if self.try_methods(&kde_methods, text)? {
                    return Ok(());
                }
            },
            "gnome" => {
                let formatted_text = format!("string:{}", text);
                let gnome_methods = [
                    (vec!["dbus-send", "--session", "--type=method_call",
                         "--dest=org.GNOME.Shell", "/org/GNOME/Shell/Clipboard",
                         "org.GNOME.Shell.Clipboard.SetText",
                         &formatted_text], "GNOME DBus"),
                ];
                if self.try_methods(&gnome_methods, text)? {
                    return Ok(());
                }
            },
            "xfce" => {
                let xfce_methods = [
                    (vec!["xfce4-clipman-cli", "-c", "--primary"], "XFCE Clipman"),
                ];
                if self.try_methods(&xfce_methods, text)? {
                    return Ok(());
                }
            },
            "mate" => {
                let mate_methods = [
                    (vec!["mate-clipboard-cmd", "copy"], "MATE Clipboard"),
                ];
                if self.try_methods(&mate_methods, text)? {
                    return Ok(());
                }
            },
            "lxde" | "lxqt" => {
                let lx_methods = [
                    (vec!["lxclipboard", "--copy"], "LX Clipboard"),
                ];
                if self.try_methods(&lx_methods, text)? {
                    return Ok(());
                }
            },
            _ => {
                if self.verbose {
                    println!("Unknown desktop environment: {}", desktop);
                }
            }
        }

        // Try generic clipboard managers
        let generic_methods = [
            (vec!["clipman", "store"], "clipman"),
            (vec!["clipcopy"], "clipcopy"),
            (vec!["clipboard-cli", "--copy"], "clipboard-cli"),
            (vec!["xclip", "-selection", "clipboard"], "xclip fallback"),
            (vec!["xsel", "-i", "-b"], "xsel fallback"),
        ];

        if self.try_methods(&generic_methods, text)? {
            return Ok(());
        }

        // Last resort: Try DBus directly
        let formatted_text = format!("string:{}", text);
        let dbus_methods = [
            // Freedesktop portal
            (vec!["dbus-send", "--session", "--type=method_call",
                 "--dest=org.freedesktop.portal.Desktop",
                 "/org/freedesktop/portal/desktop",
                 "org.freedesktop.portal.Settings.SetClipboard",
                 "string:text/plain",
                 &formatted_text], "DBus Desktop Portal"),
            // X11 fallback through xdotool
            (vec!["xdotool", "type", "--clearmodifiers", text], "xdotool"),
        ];

        if self.try_methods(&dbus_methods, text)? {
            return Ok(());
        }

        Err("Failed to copy to clipboard. Please install xclip or wl-clipboard:\n\
             For X11: sudo pacman -S xclip\n\
             For Wayland: sudo pacman -S wl-clipboard".to_string())
    }

    fn try_methods(&self, methods: &[(Vec<&str>, &str)], text: &str) -> Result<bool, String> {
        for (cmd, desc) in methods {
            if self.verbose {
                println!("Trying: {} ({})", cmd.join(" "), desc);
            }

            // Check if the command exists before trying to use it
            if Command::new(cmd[0]).arg("--version").output().is_err() {
                if self.verbose {
                    println!("Command not found: {}", cmd[0]);
                }
                continue;
            }

            let result = Command::new(cmd[0])
                .args(&cmd[1..])
                .stdin(Stdio::piped())
                .spawn()
                .and_then(|mut child| {
                    if let Some(mut stdin) = child.stdin.take() {
                        stdin.write_all(text.as_bytes())?;
                        drop(stdin);
                        child.wait().map(|status| status.success())
                    } else {
                        Ok(false)
                    }
                });

            match result {
                Ok(true) => {
                    if self.verbose {
                        println!("Successfully copied using {}", desc);
                    }
                    // Give the system a moment to process
                    thread::sleep(Duration::from_millis(100));
                    return Ok(true);
                }
                Ok(false) | Err(_) => {
                    if self.verbose {
                        println!("Failed to copy using {}", desc);
                    }
                }
            }
        }

        Ok(false)
    }
}