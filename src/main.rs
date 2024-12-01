mod cli;
mod clipboard;
mod file_processor;
mod utils;

use colored::*;
use file_processor::FileProcessor;

fn main() {
    let matches = cli::build_cli().get_matches();
    let config = cli::Config::from_matches(&matches);
    let mut processor = FileProcessor::new(config);
    
    match processor.process() {
        Ok((text_count, binary_count)) => {
            if text_count == 0 && binary_count == 0 {
                println!("{}", "No files found".yellow());
                return;
            }

            println!(
                "{} {} {} {}",
                "✨".green(),
                "Yoinked".green().bold(),
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