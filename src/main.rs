use clap::{App, Arg};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::{fs, path::PathBuf, process::{Command, Stdio}, io::Write};
use walkdir::WalkDir;

fn is_text(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }

    // Check for null bytes and non-text characters
    let text_chars = data.iter().take(512).filter(|&&b| {
        b != 0 && (b >= 32 || b == b'\n' || b == b'\r' || b == b'\t')
    }).count();

    // Consider it text if >90% of first 512 bytes are text characters
    (text_chars as f32 / data.len().min(512) as f32) > 0.9
}

fn copy_to_clipboard(text: &str, verbose: bool) -> Result<(), String> {
    let methods = [
        // KDE specific
        (vec!["qdbus", "org.kde.klipper", "/klipper", "setClipboardContents"], "KDE Klipper"),
        // Wayland
        (vec!["wl-copy"], "wl-copy"),
        // X11 methods
        (vec!["xclip", "-selection", "clipboard"], "xclip"),
        (vec!["xsel", "-i", "-b"], "xsel"),
    ];

    // Try dbus-send first as it's the fallback method you have.
    let dbus_result = Command::new("dbus-send")
        .args([
            "--type=method_call",
            "--dest=org.kde.klipper",
            "/klipper",
            "org.kde.klipper.klipper.setClipboardContents",
            format!("string:{}", text).as_str(),
        ])
        .status();

    // If dbus-send works, return success.
    if dbus_result.is_ok() && dbus_result.unwrap().success() {
        if verbose {
            println!("Successfully copied using dbus-send fallback");
        }
        return Ok(());
    }

    // Try other methods if dbus-send fails
    for (cmd, desc) in methods {
        if verbose {
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
                if verbose {
                    println!("Successfully copied using {}", desc);
                }
                return Ok(());
            }
            Ok(false) => {
                if verbose {
                    eprintln!("Failed to copy using {}", desc);
                }
            }
            Err(e) => {
                if verbose {
                    eprintln!("Error executing {}: {}", desc, e);
                }
            }
        }
    }

    // If all methods fail, return an error.
    Err("Failed to copy to clipboard".to_string())
}

fn main() {
    let matches = App::new("yoink")
        .version("0.1.0")
        .about("Quickly grab text content into your clipboard")
        .arg(
            Arg::new("path")
                .help("Directory or file to yoink")
                .default_value(".")
                .index(1)
        )
        .arg(
            Arg::new("max-size")
                .short('m')
                .long("max-size")
                .takes_value(true)
                .default_value("10")
                .help("Maximum file size in MB to consider")
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .takes_value(false)
                .help("Show verbose output")
        )
        .arg(
            Arg::new("depth")
                .short('d')
                .long("depth")
                .takes_value(true)
                .help("Maximum directory depth to traverse (0 means current directory only)")
        )
        .arg(
            Arg::new("extensions")
                .short('e')
                .long("extensions")
                .takes_value(true)
                .help("File extensions to include (comma-separated, e.g., \"txt,md,rs\")")
        )
        .arg(
            Arg::new("exclude")
                .short('x')
                .long("exclude")
                .takes_value(true)
                .help("File extensions to exclude (comma-separated)")
        )
        .arg(
            Arg::new("pattern")
                .short('p')
                .long("pattern")
                .takes_value(true)
                .help("Search pattern for filenames (supports glob patterns like *.txt)")
        )
        .arg(
            Arg::new("no-hidden")
                .short('H')
                .long("no-hidden")
                .takes_value(false)
                .help("Skip hidden files and directories")
        )
        .arg(
            Arg::new("sort")
                .short('s')
                .long("sort")
                .takes_value(false)
                .help("Sort files by name before processing")
        )
        .get_matches();

    let path = matches.value_of("path").unwrap();
    let max_size = matches.value_of("max-size")
        .unwrap()
        .parse::<u64>()
        .unwrap_or(10);
    let verbose = matches.is_present("verbose");
    let max_depth = matches.value_of("depth")
        .and_then(|d| d.parse::<u32>().ok())
        .unwrap_or(u32::MAX);
    let extensions = matches.value_of("extensions").map(|s| s.to_string());
    let exclude = matches.value_of("exclude").map(|s| s.to_string());
    let pattern = matches.value_of("pattern").map(|s| s.to_string());
    let skip_hidden = matches.is_present("no-hidden");
    let sort = matches.is_present("sort");

    let path = PathBuf::from(path);

    if !path.exists() {
        eprintln!("{}", "Error: path does not exist".red());
        std::process::exit(1);
    }

    // Parse file extensions to include/exclude
    let include_extensions: Option<Vec<String>> = extensions
        .map(|e| e.split(',').map(|s| s.trim().to_lowercase()).collect());
    let exclude_extensions: Option<Vec<String>> = exclude
        .map(|e| e.split(',').map(|s| s.trim().to_lowercase()).collect());

    // Compile glob pattern if provided
    let pattern = pattern.map(|p| glob::Pattern::new(&p).unwrap());

    // Setup progress bar
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Scanning files...");

    let mut buffer = String::new();
    let mut text_count = 0;
    let mut binary_count = 0;
    let max_size = max_size * 1024 * 1024; // Convert MB to bytes

    // Collect and possibly sort files
    let mut entries: Vec<_> = WalkDir::new(&path)
        .max_depth(max_depth as usize)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir())
        .filter(|e| {
            let path = e.path();
            let extension = path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase());

            // Check if file should be included based on all criteria
            let include_by_ext = include_extensions.as_ref()
                .map(|exts| extension.as_ref()
                    .map(|e| exts.contains(e))
                    .unwrap_or(false))
                .unwrap_or(true);

            let exclude_by_ext = exclude_extensions.as_ref()
                .map(|exts| extension.as_ref()
                    .map(|e| exts.contains(e))
                    .unwrap_or(false))
                .unwrap_or(false);

            let matches_pattern = pattern.as_ref()
                .map(|p| p.matches(path.file_name().unwrap().to_str().unwrap()))
                .unwrap_or(true);

            let is_hidden = skip_hidden && path
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.starts_with("."))
                .unwrap_or(false);

            include_by_ext && !exclude_by_ext && matches_pattern && !is_hidden
        })
        .collect();

    if sort {
        entries.sort_by_key(|e| e.path().to_path_buf());
    }

    // Add header
    buffer.push_str("=== YOINK REPORT ===\n\n");

    // Process files
    for entry in entries {
        let file_path = entry.path();
        let file_size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        
        // Skip files larger than max_size
        if file_size > max_size {
            if verbose {
                pb.println(format!("Skipping large file: {}", file_path.display()));
            }
            continue;
        }
        
        // First check if it's a known binary extension
        let ext = file_path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());
        
        let is_likely_binary = ext.map(|e| {
            matches!(e.as_str(),
                "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | 
                "wav" | "mp3" | "ogg" | "flac" |
                "pdf" | "zip" | "tar" | "gz" | "7z" |
                "exe" | "dll" | "so" | "dylib" |
                "ttf" | "otf" | "woff" | "woff2"
            )
        }).unwrap_or(false);
    
        // If it's a binary file, add its path to the clipboard buffer
        if is_likely_binary {
            if verbose {
                pb.println(format!("Found binary file: {}", file_path.display()));
            }
            
            // Just add the binary file's path to the buffer (not the content)
            buffer.push_str(&format!("\n{} (binary file)\n", file_path.display()));
            binary_count += 1;
            continue;
        }
        
        // Only try to read files that don't have binary extensions
        match fs::read(file_path) {
            Ok(content) => {
                if !is_text(&content) {
                    if verbose {
                        pb.println(format!("Found binary file: {}", file_path.display()));
                    }
                    
                    // Instead of skipping binary files, just add their paths
                    buffer.push_str(&format!("\n{} (binary file)\n", file_path.display()));
                    binary_count += 1;
                } else {
                    if verbose {
                        pb.println(format!("Processing: {}", file_path.display()));
                    }
                    
                    // Write separator and file path
                    buffer.push_str(&format!("\n{}\n", "=".repeat(50)));
                    buffer.push_str(&format!("=== {} ===\n", file_path.display()));
                    buffer.push_str(&format!("{}\n\n", "=".repeat(50)));
    
                    // Write content for text files
                    if let Ok(content_str) = String::from_utf8(content) {
                        buffer.push_str(&content_str);
                        buffer.push_str("\n\n");
                        text_count += 1;
                    }
                }
            }
            Err(e) => {
                if verbose {
                    pb.println(format!("Error reading {}: {}", file_path.display(), e));
                }
            }
        }
    }
    
    buffer.push_str("\n=== END REPORT ===\n");

    pb.finish_and_clear();

    if text_count == 0 && binary_count == 0 {
        println!("{}", "No files found".yellow());
        return;
    }

    // Try to copy to clipboard
    match copy_to_clipboard(&buffer, verbose) {
        Ok(_) => {
            println!(
                "{} {} {} {}",
                "✨".green(),
                "Processed".green().bold(),
                text_count,
                "text files!".green()
            );
            if binary_count > 0 {
                println!("Found {} binary files", binary_count);
            }
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            std::process::exit(1);
        }
    }
}
