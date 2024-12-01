// src/clipboard.rs
use std::process::{Command, Stdio};
use std::io::Write;

pub struct ClipboardManager {
    verbose: bool,
}

impl ClipboardManager {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    pub fn copy_to_clipboard(&self, text: &str) -> Result<(), String> {
        // Try universal methods first (most likely to work across systems)
        let universal_methods = [
            // Wayland
            (vec!["wl-copy"], "wl-copy"),
            (vec!["wl-clipboard"], "wl-clipboard"),
            
            // X11
            (vec!["xclip", "-selection", "clipboard"], "xclip"),
            (vec!["xsel", "-i", "-b"], "xsel"),
            
            // Generic clipboard managers
            (vec!["clipman", "store"], "clipman"),
            (vec!["clipcopy"], "clipcopy"),
            (vec!["clipboard-cli", "--copy"], "clipboard-cli"),
            (vec!["pbcopy"], "pbcopy"), // For macOS compatibility
        ];

        if self.try_methods(&universal_methods, text)? {
            return Ok(());
        }

        // Try desktop environment specific methods
        if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
            match desktop.as_str() {
                "KDE" | "plasma" | "PLASMA" => {
                    let kde_methods = vec![
                        (vec!["klipper", "-e"], "Klipper"),
                        (vec!["qdbus", "org.kde.klipper", "/klipper", "setClipboardContents"], "KDE DBus"),
                    ];
                    if self.try_methods(&kde_methods, text)? {
                        return Ok(());
                    }

                    // Try KDE DBus method separately since it needs formatted string
                    let formatted_text = format!("string:{}", text);
                    let kde_dbus_cmd = vec![
                        "dbus-send",
                        "--type=method_call",
                        "--dest=org.kde.klipper",
                        "/klipper",
                        "org.kde.klipper.klipper.setClipboardContents",
                        &formatted_text,
                    ];
                    if self.try_single_method(&kde_dbus_cmd, "KDE DBus Alt", text)? {
                        return Ok(());
                    }
                },
                "GNOME" => {
                    let gnome_methods = vec![
                        (vec!["gnome-clipboard-service", "--set"], "GNOME Clipboard Service"),
                    ];
                    if self.try_methods(&gnome_methods, text)? {
                        return Ok(());
                    }

                    // Try GNOME DBus method separately
                    let formatted_text = format!("string:{}", text);
                    let gnome_dbus_cmd = vec![
                        "dbus-send",
                        "--type=method_call",
                        "--dest=org.gnome.Shell",
                        "/org/gnome/Shell/Clipboard",
                        "org.gnome.Shell.Clipboard.SetText",
                        &formatted_text,
                    ];
                    if self.try_single_method(&gnome_dbus_cmd, "GNOME DBus", text)? {
                        return Ok(());
                    }
                },
                "XFCE" => {
                    let xfce_methods = [
                        (vec!["xfce4-clipman-cli", "-c"], "XFCE Clipman"),
                    ];
                    if self.try_methods(&xfce_methods, text)? {
                        return Ok(());
                    }
                },
                "MATE" => {
                    let mate_methods = [
                        (vec!["mate-clipboard-cmd", "copy"], "MATE Clipboard"),
                    ];
                    if self.try_methods(&mate_methods, text)? {
                        return Ok(());
                    }
                },
                _ => {}
            }
        }

        // Last resort: Try direct DBus method
        let formatted_text = format!("string:{}", text);
        let dbus_portal_cmd = vec![
            "dbus-send",
            "--type=method_call",
            "--dest=org.freedesktop.Portal.Desktop",
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.Settings.SetClipboard",
            "string:text/plain",
            &formatted_text,
        ];
        if self.try_single_method(&dbus_portal_cmd, "DBus Desktop Portal", text)? {
            return Ok(());
        }

        Err("Failed to copy to clipboard - no compatible clipboard program found. Please install xclip, wl-clipboard, or another clipboard manager.".to_string())
    }

    fn try_single_method(&self, cmd: &[&str], desc: &str, text: &str) -> Result<bool, String> {
        if self.verbose {
            println!("Trying: {} ({})", cmd.join(" "), desc);
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
                Ok(true)
            }
            Ok(false) | Err(_) => {
                if self.verbose {
                    println!("Failed to copy using {}", desc);
                }
                Ok(false)
            }
        }
    }

    fn try_methods(&self, methods: &[(Vec<&str>, &str)], text: &str) -> Result<bool, String> {
        for (cmd, desc) in methods {
            if self.verbose {
                println!("Trying: {} ({})", cmd.join(" "), desc);
            }

            let result = Command::new(&cmd[0])
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