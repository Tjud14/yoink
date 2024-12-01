use clap::{Command, Arg};

pub struct Config {
    pub path: String,
    pub max_size: u64,
    pub verbose: bool,
    pub max_depth: u32,
    pub include_extensions: Option<Vec<String>>,
    pub exclude_extensions: Option<Vec<String>>,
    pub exclude_paths: Option<Vec<String>>,
    pub pattern: Option<glob::Pattern>,
    pub skip_hidden: bool,
    pub sort: bool,
}

impl Config {
    pub fn from_matches(matches: &clap::ArgMatches) -> Self {
        Config {
            path: matches.get_one::<String>("path").unwrap().clone(),
            max_size: matches.get_one::<String>("max-size")
                .unwrap()
                .parse::<u64>()
                .unwrap_or(10) * 1024 * 1024,
            verbose: matches.get_flag("verbose"),
            max_depth: matches.get_one::<String>("depth")
                .and_then(|d| d.parse::<u32>().ok())
                .unwrap_or(u32::MAX),
            include_extensions: matches.get_one::<String>("extensions")
                .map(|e| e.split(',').map(|s| s.trim().to_lowercase()).collect()),
            exclude_extensions: matches.get_one::<String>("exclude")
                .map(|e| e.split(',').map(|s| s.trim().to_lowercase()).collect()),
            exclude_paths: matches.get_one::<String>("exclude-paths")
                .map(|p| p.split(',').map(|s| s.trim().to_string()).collect()),
            pattern: matches.get_one::<String>("pattern")
                .map(|p| glob::Pattern::new(p).unwrap()),
            skip_hidden: matches.get_flag("no-hidden"),
            sort: matches.get_flag("sort"),
        }
    }
}

pub fn build_cli() -> Command {
    Command::new("yoink")
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
                .value_name("SIZE")
                .default_value("10")
                .help("Maximum file size in MB to consider")
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(clap::ArgAction::SetTrue)
                .help("Show verbose output")
        )
        .arg(
            Arg::new("depth")
                .short('d')
                .long("depth")
                .value_name("DEPTH")
                .help("Maximum directory depth to traverse (0 means current directory only)")
        )
        .arg(
            Arg::new("extensions")
                .short('e')
                .long("extensions")
                .value_name("EXTS")
                .help("File extensions to include (comma-separated, e.g., \"txt,md,rs\")")
        )
        .arg(
            Arg::new("exclude")
                .short('x')
                .long("exclude")
                .value_name("EXTS")
                .help("File extensions to exclude (comma-separated)")
        )
        .arg(
            Arg::new("exclude-paths")
                .long("exclude-paths")
                .value_name("PATHS")
                .help("Paths to exclude (comma-separated)")
        )
        .arg(
            Arg::new("pattern")
                .short('p')
                .long("pattern")
                .value_name("PATTERN")
                .help("Search pattern for filenames (supports glob patterns like *.txt)")
        )
        .arg(
            Arg::new("no-hidden")
                .short('H')
                .long("no-hidden")
                .action(clap::ArgAction::SetTrue)
                .help("Skip hidden files and directories")
        )
        .arg(
            Arg::new("sort")
                .short('s')
                .long("sort")
                .action(clap::ArgAction::SetTrue)
                .help("Sort files by name before processing")
        )
}