use clap::Parser;
use globset::{Glob, GlobSet, GlobSetBuilder};
use humantime::Duration;
use regex::Regex;
use serde::Serialize;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::SystemTime;
use walkdir::WalkDir;

#[cfg(windows)]
use windows_sys::Win32::Foundation::*;
#[cfg(windows)]
use windows_sys::Win32::System::Console::*;

#[cfg(windows)]
fn enable_vt100() {
    unsafe {
        let stdout = GetStdHandle(STD_OUTPUT_HANDLE);
        if stdout != INVALID_HANDLE_VALUE {
            let mut mode = 0u32;
            if GetConsoleMode(stdout, &mut mode) != 0 {
                mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING;
                mode |= DISABLE_NEWLINE_AUTO_RETURN;
                let _ = SetConsoleMode(stdout, mode);
            }
        }
        let stderr = GetStdHandle(STD_ERROR_HANDLE);
        if stderr != INVALID_HANDLE_VALUE {
            let mut mode = 0u32;
            if GetConsoleMode(stderr, &mut mode) != 0 {
                mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING;
                let _ = SetConsoleMode(stderr, mode);
            }
        }
    }
}

#[cfg(not(windows))]
fn enable_vt100() {}

#[derive(Parser, Debug)]
#[command(name = "rsdata")]
#[command(about = "Recursive file search with content matching", long_about = None)]
#[command(color = clap::ColorChoice::Auto)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Directory to search in
    #[arg(default_value = ".")]
    directory: String,

    /// Content pattern to search for
    #[arg(long)]
    pattern: Option<String>,

    /// Additional pattern to search (can be used multiple times; OR logic)
    #[arg(short = 'e', long = "regexp")]
    patterns: Vec<String>,

    /// Read patterns from file, one per line
    #[arg(short = 'f', long = "file")]
    pattern_files: Vec<String>,

    /// Treat pattern as fixed string (not regex)
    #[arg(short = 'F', long = "fixed-strings")]
    fixed_strings: bool,

    /// Match whole words only
    #[arg(short = 'w', long = "word-regexp")]
    word_regexp: bool,

    /// Match entire lines only
    #[arg(short = 'x', long = "line-regexp")]
    line_regexp: bool,

    /// Invert match (show lines/files that do NOT match)
    #[arg(short = 'v', long = "invert-match")]
    invert_match: bool,

    /// Case-sensitive search (default: smart-case)
    #[arg(short = 's', long = "case-sensitive")]
    case_sensitive: bool,

    /// Case-insensitive search
    #[arg(short = 'i', long = "ignore-case")]
    ignore_case: bool,

    /// Smart case: case-insensitive unless pattern contains uppercase
    #[arg(short = 'S', long = "smart-case")]
    smart_case: bool,

    /// Treat pattern as regex (default unless -F)
    #[arg(long)]
    regex: bool,

    /// Filter files by glob pattern (use !prefix to exclude)
    #[arg(short = 'g', long = "glob", value_delimiter = ',')]
    globs: Vec<String>,

    /// Filter by file type alias (e.g., -t rs, -t py)
    #[arg(short = 't', long = "type", value_delimiter = ',')]
    types: Vec<String>,

    /// Exclude by file type alias
    #[arg(short = 'T', long = "type-not", value_delimiter = ',')]
    types_not: Vec<String>,

    /// File extensions to include
    #[arg(short = 'E', long = "extension", value_delimiter = ',')]
    extensions: Vec<String>,

    /// Exclude file extensions
    #[arg(long = "exclude-extension", value_delimiter = ',')]
    exclude_extensions: Vec<String>,

    /// Exclude directories
    #[arg(long = "exclude-dir", value_delimiter = ',')]
    exclude_dirs: Vec<String>,

    /// Number of lines to show after each match
    #[arg(short = 'A', long = "after-context")]
    after_context: Option<usize>,

    /// Number of lines to show before each match
    #[arg(short = 'B', long = "before-context")]
    before_context: Option<usize>,

    /// Number of lines to show before and after each match (-B N -A N)
    #[arg(short = 'C', long = "context")]
    context: Option<usize>,

    /// Show line numbers
    #[arg(short = 'n', long = "line-number")]
    line_number: bool,

    /// Don't show line numbers
    #[arg(long = "no-line-number")]
    no_line_number: bool,

    /// Show only matched files (not content)
    #[arg(short = 'l', long = "files-with-matches")]
    files_with_matches: bool,

    /// Show only matched files (no matches)
    #[arg(long = "files-without-match")]
    files_without_match: bool,

    /// Show count of matches per file
    #[arg(short = 'c', long = "count")]
    count: bool,

    /// Show only the matched parts of lines
    #[arg(short = 'o', long = "only-matching")]
    only_matching: bool,

    /// Prefix with file name
    #[arg(short = 'H', long = "with-filename")]
    with_filename: bool,

    /// Don't prefix with file name
    #[arg(long = "no-filename")]
    no_filename: bool,

    /// Group matches under file headings
    #[arg(long = "heading")]
    heading: bool,

    /// Don't show headings
    #[arg(long = "no-heading")]
    no_heading: bool,

    /// Print ALL lines, highlight matches
    #[arg(long = "passthru")]
    passthru: bool,

    /// Treat binary files as text
    #[arg(short = 'a', long = "text")]
    text: bool,

    /// Include hidden files and directories
    #[arg(long = "hidden")]
    hidden: bool,

    /// Respect .gitignore files
    #[arg(long = "ignore")]
    ignore: bool,

    /// Don't respect .gitignore/.ignore files
    #[arg(short = 'u', long = "no-ignore")]
    no_ignore: bool,

    /// Follow symbolic links
    #[arg(short = 'L', long = "follow")]
    follow: bool,

    /// Maximum depth to search
    #[arg(short = 'd', long = "max-depth")]
    max_depth: Option<usize>,

    /// Maximum matches per file
    #[arg(short = 'm', long = "max-count")]
    max_count: Option<usize>,

    /// Filter by minimum file size (+N, N, or with suffix: k, M, G, T)
    #[arg(long = "min-size")]
    min_size: Option<String>,

    /// Filter by maximum file size (-N or with suffix: k, M, G, T)
    #[arg(long = "max-size")]
    max_size: Option<String>,

    /// Files modified within duration (e.g., 1h, 30min, 2d, 1week)
    #[arg(long = "changed-within")]
    changed_within: Option<Duration>,

    /// Files modified before duration (e.g., 2d = older than 2 days)
    #[arg(long = "changed-before")]
    changed_before: Option<Duration>,

    /// Just list files without searching content
    #[arg(long = "files")]
    files_only: bool,

    /// Output in JSON format
    #[arg(long = "json")]
    json: bool,

    /// Print statistics
    #[arg(long = "stats")]
    stats: bool,

    /// Quiet mode: exit code only
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,

    /// Use null byte as separator
    #[arg(short = '0', long = "null")]
    print0: bool,

    /// Show absolute paths
    #[arg(long = "absolute-path")]
    absolute_path: bool,

    /// Color mode
    #[arg(long = "color", value_enum, default_value = "auto")]
    color: ColorWhen,
}

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
enum ColorWhen {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone)]
struct FileTypeDef {
    name: String,
    globs: Vec<String>,
}

#[derive(Serialize, Debug)]
struct MatchInfo {
    path: String,
    line: Option<usize>,
    content: Option<String>,
    matches: Vec<String>,
}

#[derive(Serialize, Debug)]
struct FileResult {
    path: String,
    matches: usize,
    lines: Vec<MatchInfo>,
}

fn builtin_file_types() -> Vec<FileTypeDef> {
    vec![
        FileTypeDef {
            name: "rust".into(),
            globs: vec!["*.rs".into()],
        },
        FileTypeDef {
            name: "rs".into(),
            globs: vec!["*.rs".into()],
        },
        FileTypeDef {
            name: "python".into(),
            globs: vec!["*.py".into()],
        },
        FileTypeDef {
            name: "py".into(),
            globs: vec!["*.py".into()],
        },
        FileTypeDef {
            name: "js".into(),
            globs: vec!["*.js".into(), "*.jsx".into()],
        },
        FileTypeDef {
            name: "ts".into(),
            globs: vec!["*.ts".into(), "*.tsx".into()],
        },
        FileTypeDef {
            name: "html".into(),
            globs: vec!["*.html".into(), "*.htm".into()],
        },
        FileTypeDef {
            name: "css".into(),
            globs: vec!["*.css".into(), "*.scss".into(), "*.sass".into(), "*.less".into()],
        },
        FileTypeDef {
            name: "json".into(),
            globs: vec!["*.json".into()],
        },
        FileTypeDef {
            name: "toml".into(),
            globs: vec!["*.toml".into()],
        },
        FileTypeDef {
            name: "yaml".into(),
            globs: vec!["*.yaml".into(), "*.yml".into()],
        },
        FileTypeDef {
            name: "md".into(),
            globs: vec!["*.md".into(), "*.markdown".into()],
        },
        FileTypeDef {
            name: "c".into(),
            globs: vec!["*.c".into(), "*.h".into()],
        },
        FileTypeDef {
            name: "cpp".into(),
            globs: vec!["*.cpp".into(), "*.cc".into(), "*.hpp".into(), "*.h".into()],
        },
        FileTypeDef {
            name: "java".into(),
            globs: vec!["*.java".into()],
        },
        FileTypeDef {
            name: "go".into(),
            globs: vec!["*.go".into()],
        },
        FileTypeDef {
            name: "rb".into(),
            globs: vec!["*.rb".into()],
        },
        FileTypeDef {
            name: "php".into(),
            globs: vec!["*.php".into()],
        },
        FileTypeDef {
            name: "sh".into(),
            globs: vec!["*.sh".into(), "*.bash".into()],
        },
        FileTypeDef {
            name: "text".into(),
            globs: vec!["*.txt".into()],
        },
    ]
}

fn parse_patterns(cli: &Cli) -> Vec<String> {
    let mut pats = Vec::new();

    if let Some(p) = &cli.pattern {
        pats.push(p.clone());
    }

    for p in &cli.patterns {
        pats.push(p.clone());
    }

    for file_path in &cli.pattern_files {
        if let Ok(f) = File::open(file_path) {
            let reader = BufReader::new(f);
            for line in reader.lines() {
                if let Ok(l) = line {
                    if !l.is_empty() {
                        pats.push(l);
                    }
                }
            }
        }
    }

    pats
}

fn build_regex_set(patterns: &[String], cli: &Cli) -> Result<Vec<Regex>, String> {
    let mut regexes = Vec::new();

    let case_override = if cli.ignore_case {
        true
    } else if cli.case_sensitive {
        false
    } else if cli.smart_case {
        !patterns.iter().any(|p| p.chars().any(|c| c.is_uppercase()))
    } else {
        true
    };

    for pat in patterns {
        let regex_str = if cli.fixed_strings {
            regex::escape(pat)
        } else {
            pat.clone()
        };

        let regex_str = if cli.word_regexp {
            format!(r"\b{}\b", regex_str)
        } else if cli.line_regexp {
            format!(r"^{}$", regex_str)
        } else {
            regex_str
        };

        let regex_str = if case_override && !cli.case_sensitive {
            format!("(?i){}", regex_str)
        } else {
            regex_str
        };

        let re = Regex::new(&regex_str).map_err(|e| e.to_string())?;
        regexes.push(re);
    }

    Ok(regexes)
}

fn build_glob_set(globs: &[String]) -> Result<(GlobSet, GlobSet), String> {
    let mut include_builder = GlobSetBuilder::new();
    let mut exclude_builder = GlobSetBuilder::new();

    for g in globs {
        if let Some(stripped) = g.strip_prefix('!') {
            let glob = Glob::new(stripped).map_err(|e| e.to_string())?;
            exclude_builder.add(glob);
        } else {
            let glob = Glob::new(g).map_err(|e| e.to_string())?;
            include_builder.add(glob);
        }
    }

    Ok((
        include_builder.build().map_err(|e| e.to_string())?,
        exclude_builder.build().map_err(|e| e.to_string())?,
    ))
}

fn type_to_globs(types: &[String], include: bool) -> Vec<String> {
    let mut globs = Vec::new();
    let builtin = builtin_file_types();

    for t in types {
        for def in &builtin {
            if &def.name == t {
                for g in &def.globs {
                    if include {
                        globs.push(g.clone());
                    } else {
                        globs.push(format!("!{}", g));
                    }
                }
            }
        }
    }

    globs
}

fn parse_size(size_str: &str) -> Result<u64, String> {
    let s = size_str.trim_start_matches(&['+', '-']);
    let (num_part, suffix): (String, String) = s.chars().partition(|c| c.is_ascii_digit());

    let num: u64 = num_part.parse().map_err(|_| "Invalid size number")?;

    let multiplier = match suffix.to_lowercase().as_str() {
        "" => 1,
        "b" | "c" => 1,
        "k" => 1024,
        "m" => 1024 * 1024,
        "g" => 1024 * 1024 * 1024,
        "t" => 1024 * 1024 * 1024 * 1024,
        _ => return Err("Invalid size suffix".into()),
    };

    Ok(num * multiplier)
}

fn is_dir_excluded(name: &str, cli: &Cli) -> bool {
    if name == "." || name == ".." {
        return false;
    }
    if !cli.hidden && name.starts_with('.') {
        return true;
    }
    if cli.exclude_dirs.iter().any(|d| d == name) {
        return true;
    }
    false
}

fn file_passes_filters(path: &Path, metadata: &fs::Metadata, cli: &Cli) -> bool {
    if !cli.extensions.is_empty() {
        let allowed = path
            .extension()
            .map(|e| cli.extensions.iter().any(|ext| ext == &e.to_string_lossy()))
            .unwrap_or(false);
        if !allowed {
            return false;
        }
    }

    if !cli.exclude_extensions.is_empty() {
        let excluded = path
            .extension()
            .map(|e| {
                cli.exclude_extensions
                    .iter()
                    .any(|ext| ext == &e.to_string_lossy())
            })
            .unwrap_or(false);
        if excluded {
            return false;
        }
    }

    if let Some(ref min) = cli.min_size {
        if let Ok(min_bytes) = parse_size(min) {
            if metadata.len() < min_bytes {
                return false;
            }
        }
    }

    if let Some(ref max) = cli.max_size {
        if let Ok(max_bytes) = parse_size(max) {
            if metadata.len() > max_bytes {
                return false;
            }
        }
    }

    if let Some(ref within) = cli.changed_within {
        if let Ok(mtime) = metadata.modified() {
            let now = SystemTime::now();
            if let Ok(diff) = now.duration_since(mtime) {
                if diff > *within.as_ref() {
                    return false;
                }
            }
        }
    }

    if let Some(ref before) = cli.changed_before {
        if let Ok(mtime) = metadata.modified() {
            let now = SystemTime::now();
            if let Ok(diff) = now.duration_since(mtime) {
                if diff < *before.as_ref() {
                    return false;
                }
            }
        }
    }

    true
}

fn match_content(content: &str, regexes: &[Regex], cli: &Cli) -> (bool, Vec<String>) {
    let mut matches = Vec::new();

    for re in regexes {
        for cap in re.captures_iter(content) {
            if let Some(m) = cap.get(0) {
                matches.push(m.as_str().to_string());
            }
        }
    }

    let is_match = !matches.is_empty();

    if cli.invert_match {
        (!is_match, Vec::new())
    } else {
        (is_match, matches)
    }
}

fn format_path(path: &Path, cli: &Cli) -> String {
    if cli.absolute_path {
        if let Ok(abs) = path.canonicalize() {
            return abs.to_string_lossy().to_string();
        }
    }
    path.to_string_lossy().to_string()
}

fn main() {
    enable_vt100();
    let cli = Cli::parse();

    let search_dir = Path::new(&cli.directory);
    if !search_dir.exists() {
        eprintln!("Error: Directory '{}' does not exist", cli.directory);
        std::process::exit(1);
    }

    let patterns = parse_patterns(&cli);
    let do_content_search = !patterns.is_empty() && !cli.files_only;

    let regexes = if do_content_search {
        match build_regex_set(&patterns, &cli) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Regex error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        Vec::new()
    };

    let mut all_globs = cli.globs.clone();
    all_globs.extend(type_to_globs(&cli.types, true));
    all_globs.extend(type_to_globs(&cli.types_not, false));

    let (include_globs, exclude_globs) = if !all_globs.is_empty() {
        match build_glob_set(&all_globs) {
            Ok((i, e)) => (Some(i), Some(e)),
            Err(e) => {
                eprintln!("Glob error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        (None, None)
    };

    let mut file_count: usize = 0;
    let mut match_count: usize = 0;
    let mut file_match_count: usize = 0;
    let mut json_results: Vec<FileResult> = Vec::new();

    let before = cli.before_context.or(cli.context).unwrap_or(0);
    let after = cli.after_context.or(cli.context).unwrap_or(0);

    let mut walker = WalkDir::new(search_dir);
    if let Some(depth) = cli.max_depth {
        walker = walker.max_depth(depth);
    }
    if cli.follow {
        walker = walker.follow_links(true);
    }

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if entry.file_type().is_dir() {
            continue;
        }

        if !entry.file_type().is_file() {
            continue;
        }

        let excluded_by_path = path.components().any(|comp| {
            comp.as_os_str()
                .to_str()
                .map(|s| is_dir_excluded(s, &cli))
                .unwrap_or(false)
        });
        if excluded_by_path {
            continue;
        }

        if let Some(ref globs) = exclude_globs {
            if !globs.is_empty() && globs.is_match(path) {
                continue;
            }
        }

        if let Some(ref globs) = include_globs {
            if !globs.is_empty() && !globs.is_match(path) {
                continue;
            }
        }

        let metadata = match fs::metadata(path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if !file_passes_filters(path, &metadata, &cli) {
            continue;
        }

        file_count += 1;
        let path_str = format_path(path, &cli);

        if cli.files_only || !do_content_search {
            if cli.json {
                json_results.push(FileResult {
                    path: path_str.clone(),
                    matches: 0,
                    lines: Vec::new(),
                });
            } else if cli.files_with_matches || cli.files_without_match {
                if !cli.quiet {
                    if cli.print0 {
                        print!("{}\0", path_str);
                    } else {
                        println!("{}", path_str);
                    }
                }
                continue;
            } else {
                if !cli.quiet {
                    if cli.print0 {
                        print!("{}\0", path_str);
                    } else {
                        println!("{}", path_str);
                    }
                }
                continue;
            }
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let lines: Vec<&str> = content.lines().collect();
        let mut file_matches: Vec<(usize, String, Vec<String>)> = Vec::new();

        let mut stop = false;
        let mut matched_indices = Vec::new();

        for (idx, line) in lines.iter().enumerate() {
            let (is_match, text_matches) = match_content(line, &regexes, &cli);

            if is_match {
                if let Some(max) = cli.max_count {
                    if file_matches.len() >= max {
                        stop = true;
                    }
                }

                if !stop {
                    file_matches.push((idx + 1, line.to_string(), text_matches));
                    matched_indices.push(idx);
                }
            }
        }

        let has_match = !file_matches.is_empty();
        let should_report = if cli.files_without_match {
            !has_match
        } else {
            has_match
        };

        if !should_report {
            continue;
        }

        file_match_count += 1;
        match_count += file_matches.len();

        if cli.json {
            let mut line_infos = Vec::new();
            for (ln, line_content, _) in &file_matches {
                line_infos.push(MatchInfo {
                    path: path_str.clone(),
                    line: Some(*ln),
                    content: Some(line_content.clone()),
                    matches: Vec::new(),
                });
            }
            json_results.push(FileResult {
                path: path_str.clone(),
                matches: file_matches.len(),
                lines: line_infos,
            });
        }

        if cli.json {
            continue;
        }

        if cli.quiet {
            continue;
        }

        if cli.files_with_matches || cli.files_without_match {
            if cli.print0 {
                print!("{}\0", path_str);
            } else {
                println!("{}", path_str);
            }
            continue;
        }

        if cli.count {
            println!("{}: {}", path_str, file_matches.len());
            continue;
        }

        if !cli.json {
            let show_filename = if cli.no_filename {
                false
            } else if cli.with_filename {
                true
            } else {
                true
            };

            let show_heading = if cli.no_heading {
                false
            } else if cli.heading {
                true
            } else {
                false
            };

            if show_filename || show_heading {
                println!("\x1b[1;32m{}\x1b[0m", path_str);
            }

            if cli.passthru {
                for (idx, line) in lines.iter().enumerate() {
                    if cli.line_number && !cli.no_line_number {
                        print!("\x1b[1;36m{}\x1b[0m: ", idx + 1);
                    }
                    println!("{}", line);
                }
            } else {
                for (idx, line, text_matches) in &file_matches {
                    let show_line_number = if cli.no_line_number {
                        false
                    } else {
                        true
                    };

                    if before > 0 || after > 0 {
                        let start = if *idx > before { idx - before - 1 } else { 0 };
                        let end = (idx + after).min(lines.len());

                        for ctx_idx in start..=end.saturating_sub(1) {
                            let is_match_ctx = matched_indices.contains(&ctx_idx);
                            let ctx_line = lines[ctx_idx];

                            if is_match_ctx {
                                if show_line_number {
                                    print!("\x1b[1;36m{}\x1b[0m: ", ctx_idx + 1);
                                }
                                let highlighted = if cli.only_matching {
                                    if text_matches.is_empty() {
                                        ctx_line.to_string()
                                    } else {
                                        text_matches.join("\n")
                                    }
                                } else {
                                    ctx_line.to_string()
                                };
                                println!("{}", highlighted);
                            } else {
                                if show_line_number {
                                    print!("\x1b[90m{}\x1b[0m- ", ctx_idx + 1);
                                }
                                println!("\x1b[90m{}\x1b[0m", ctx_line);
                            }
                        }
                    } else {
                        if show_line_number {
                            print!("\x1b[1;36m{}\x1b[0m: ", idx);
                        }

                        if cli.only_matching {
                            for m in text_matches {
                                println!("{}", m);
                            }
                        } else {
                            println!("{}", line);
                        }
                    }
                }
            }
        }
    }

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&json_results).unwrap());
    }

    if cli.stats && !cli.quiet {
        if !json_results.is_empty() || file_count > 0 {
            println!();
        }
        println!("{}", "-".repeat(50));
        println!("Files scanned: {}", file_count);
        if do_content_search {
            println!("Files matched: {}", file_match_count);
            println!("Total matches: {}", match_count);
        } else {
            println!("Files found: {}", file_match_count.max(json_results.len()));
        }
    }

    if cli.quiet && match_count > 0 {
        std::process::exit(0);
    } else if cli.quiet {
        std::process::exit(1);
    }
}
