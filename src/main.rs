mod search;

use clap::Parser;
use humantime::Duration;
use std::fs;
use std::io::IsTerminal;
use std::path::Path;
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
pub struct Cli {
    /// Directory to search in
    #[arg(default_value = ".")]
    pub directory: String,

    /// Content pattern to search for
    #[arg(short = 'p', long = "pattern")]
    pub pattern: Option<String>,

    /// Additional pattern to search (can be used multiple times; OR logic)
    #[arg(short = 'e', long = "regexp")]
    pub patterns: Vec<String>,

    /// Read patterns from file, one per line
    #[arg(short = 'f', long = "file")]
    pub pattern_files: Vec<String>,

    /// Treat pattern as fixed string (not regex)
    #[arg(short = 'F', long = "fixed-strings")]
    pub fixed_strings: bool,

    /// Match whole words only
    #[arg(short = 'w', long = "word-regexp")]
    pub word_regexp: bool,

    /// Match entire lines only
    #[arg(short = 'x', long = "line-regexp")]
    pub line_regexp: bool,

    /// Invert match (show lines/files that do NOT match)
    #[arg(short = 'v', long = "invert-match")]
    pub invert_match: bool,

    /// Case-sensitive search (default: smart-case)
    #[arg(short = 's', long = "case-sensitive")]
    pub case_sensitive: bool,

    /// Case-insensitive search
    #[arg(short = 'i', long = "ignore-case")]
    pub ignore_case: bool,

    /// Smart case: case-insensitive unless pattern contains uppercase
    #[arg(short = 'S', long = "smart-case")]
    pub smart_case: bool,

    /// Treat pattern as regex (default unless -F)
    #[arg(long)]
    pub regex: bool,

    /// Filter files by glob pattern (use !prefix to exclude)
    #[arg(short = 'g', long = "glob", value_delimiter = ',')]
    pub globs: Vec<String>,

    /// Filter by file type alias (e.g., -t rs, -t py)
    #[arg(short = 't', long = "type", value_delimiter = ',')]
    pub types: Vec<String>,

    /// Exclude by file type alias
    #[arg(short = 'T', long = "type-not", value_delimiter = ',')]
    pub types_not: Vec<String>,

    /// File extensions to include
    #[arg(short = 'E', long = "extension", value_delimiter = ',')]
    pub extensions: Vec<String>,

    /// Exclude file extensions
    #[arg(long = "exclude-extension", value_delimiter = ',')]
    pub exclude_extensions: Vec<String>,

    /// Exclude directories
    #[arg(long = "exclude-dir", value_delimiter = ',')]
    pub exclude_dirs: Vec<String>,

    /// Number of lines to show after each match
    #[arg(short = 'A', long = "after-context")]
    pub after_context: Option<usize>,

    /// Number of lines to show before each match
    #[arg(short = 'B', long = "before-context")]
    pub before_context: Option<usize>,

    /// Number of lines to show before and after each match (-B N -A N)
    #[arg(short = 'C', long = "context")]
    pub context: Option<usize>,

    /// Show line numbers
    #[arg(short = 'n', long = "line-number")]
    pub line_number: bool,

    /// Don't show line numbers
    #[arg(long = "no-line-number")]
    pub no_line_number: bool,

    /// Show only matched files (not content)
    #[arg(short = 'l', long = "files-with-matches")]
    pub files_with_matches: bool,

    /// Show only matched files (no matches)
    #[arg(long = "files-without-match")]
    pub files_without_match: bool,

    /// Show count of matches per file
    #[arg(short = 'c', long = "count")]
    pub count: bool,

    /// Show only the matched parts of lines
    #[arg(short = 'o', long = "only-matching")]
    pub only_matching: bool,

    /// Prefix with file name
    #[arg(short = 'H', long = "with-filename")]
    pub with_filename: bool,

    /// Don't prefix with file name
    #[arg(long = "no-filename")]
    pub no_filename: bool,

    /// Group matches under file headings
    #[arg(long = "heading")]
    pub heading: bool,

    /// Don't show headings
    #[arg(long = "no-heading")]
    pub no_heading: bool,

    /// Print ALL lines, highlight matches
    #[arg(long = "passthru")]
    pub passthru: bool,

    /// Treat binary files as text
    #[arg(short = 'a', long = "text")]
    pub text: bool,

    /// Include hidden files and directories
    #[arg(long = "hidden")]
    pub hidden: bool,

    /// Respect .gitignore files
    #[arg(long = "ignore")]
    pub ignore: bool,

    /// Don't respect .gitignore/.ignore files
    #[arg(short = 'u', long = "no-ignore")]
    pub no_ignore: bool,

    /// Follow symbolic links
    #[arg(short = 'L', long = "follow")]
    pub follow: bool,

    /// Maximum depth to search
    #[arg(short = 'd', long = "max-depth")]
    pub max_depth: Option<usize>,

    /// Maximum matches per file
    #[arg(short = 'm', long = "max-count")]
    pub max_count: Option<usize>,

    /// Filter by minimum file size (+N, N, or with suffix: k, M, G, T)
    #[arg(long = "min-size")]
    pub min_size: Option<String>,

    /// Filter by maximum file size (-N or with suffix: k, M, G, T)
    #[arg(long = "max-size")]
    pub max_size: Option<String>,

    /// Files modified within duration (e.g., 1h, 30min, 2d, 1week)
    #[arg(long = "changed-within")]
    pub changed_within: Option<Duration>,

    /// Files modified before duration (e.g., 2d = older than 2 days)
    #[arg(long = "changed-before")]
    pub changed_before: Option<Duration>,

    /// Just list files without searching content
    #[arg(long = "files")]
    pub files_only: bool,

    /// Output in JSON format
    #[arg(long = "json")]
    pub json: bool,

    /// Print statistics
    #[arg(long = "stats")]
    pub stats: bool,

    /// Quiet mode: exit code only
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,

    /// Use null byte as separator
    #[arg(short = '0', long = "null")]
    pub print0: bool,

    /// Show absolute paths
    #[arg(long = "absolute-path")]
    pub absolute_path: bool,

    /// Color mode
    #[arg(long = "color", value_enum, default_value = "auto")]
    pub color: ColorWhen,
}

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum ColorWhen {
    Auto,
    Always,
    Never,
}

fn use_color(cli: &Cli) -> bool {
    match cli.color {
        ColorWhen::Always => true,
        ColorWhen::Never => false,
        ColorWhen::Auto => std::io::stdout().is_terminal(),
    }
}

fn fmt_green(s: &str, color: bool) -> String {
    if color { format!("\x1b[1;32m{}\x1b[0m", s) } else { s.to_string() }
}

fn fmt_cyan(s: &str, color: bool) -> String {
    if color { format!("\x1b[1;36m{}\x1b[0m", s) } else { s.to_string() }
}

fn fmt_gray(s: &str, color: bool) -> String {
    if color { format!("\x1b[90m{}\x1b[0m", s) } else { s.to_string() }
}

fn main() {
    enable_vt100();
    let cli = Cli::parse();
    let color = use_color(&cli);

    let search_dir = Path::new(&cli.directory);
    if !search_dir.exists() {
        eprintln!("Error: Directory '{}' does not exist", cli.directory);
        std::process::exit(1);
    }

    let patterns = search::parse_patterns(&cli);
    let do_content_search = !patterns.is_empty() && !cli.files_only;

    let regexes = if do_content_search {
        match search::build_regex_set(&patterns, &cli) {
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
    all_globs.extend(search::type_to_globs(&cli.types, true));
    all_globs.extend(search::type_to_globs(&cli.types_not, false));

    let (include_globs, exclude_globs) = if !all_globs.is_empty() {
        match search::build_glob_set(&all_globs) {
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
    let mut json_results: Vec<search::FileResult> = Vec::new();

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
                .map(|s| search::is_dir_excluded(s, &cli))
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

        if !search::file_passes_filters(path, &metadata, &cli) {
            continue;
        }

        file_count += 1;
        let path_str = search::format_path(path, &cli);

        if cli.files_only || !do_content_search {
            if cli.json {
                json_results.push(search::FileResult {
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
            let (is_match, text_matches) = search::match_content(line, &regexes, &cli);

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
                line_infos.push(search::MatchInfo {
                    path: path_str.clone(),
                    line: Some(*ln),
                    content: Some(line_content.clone()),
                    matches: Vec::new(),
                });
            }
            json_results.push(search::FileResult {
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
            let show_filename = if cli.no_filename || cli.heading {
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

            if show_heading {
                println!("{}", fmt_green(&format!("{} ", path_str), color));
                println!("{}", "-".repeat(path_str.len() + 1));
            } else if show_filename {
                println!("{}", fmt_green(&path_str, color));
            }

                if cli.passthru {
                for (idx, line) in lines.iter().enumerate() {
                    if cli.line_number && !cli.no_line_number {
                        print!("{}: ", fmt_cyan(&(idx + 1).to_string(), color));
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
                        let mut ranges: Vec<(usize, usize)> = Vec::new();
                        for (mi, _, _) in &file_matches {
                            let start = if *mi > before { mi - before - 1 } else { 0 };
                            let end = (*mi + after).min(lines.len());
                            ranges.push((start, end));
                        }
                        ranges.sort();
                        let mut merged: Vec<(usize, usize)> = Vec::new();
                        for (s, e) in ranges {
                            if let Some(last) = merged.last_mut() {
                                if s <= last.1 {
                                    last.1 = last.1.max(e);
                                } else {
                                    merged.push((s, e));
                                }
                            } else {
                                merged.push((s, e));
                            }
                        }
                        for (block_idx, (start, end)) in merged.iter().enumerate() {
                            if block_idx > 0 {
                                println!("--");
                            }
                            for ctx_idx in *start..*end {
                                let is_match_ctx = matched_indices.contains(&ctx_idx);
                                let ctx_line = lines[ctx_idx];

                                if is_match_ctx {
                                    if show_line_number {
                                        print!("{}: ", fmt_cyan(&(ctx_idx + 1).to_string(), color));
                                    }
                                    let highlighted = if cli.only_matching {
                                        let empty = Vec::new();
                                        let text_matches_for_line = file_matches.iter().find(|(mi, _, _)| *mi == ctx_idx + 1).map(|(_, _, tm)| tm).unwrap_or(&empty);
                                        if text_matches_for_line.is_empty() {
                                            ctx_line.to_string()
                                        } else {
                                            text_matches_for_line.join("\n")
                                        }
                                    } else {
                                        ctx_line.to_string()
                                    };
                                    println!("{}", highlighted);
                                } else {
                                    if show_line_number {
                                        print!("{}- ", fmt_gray(&(ctx_idx + 1).to_string(), color));
                                    }
                                    println!("{}", fmt_gray(ctx_line, color));
                                }
                            }
                        }
                        if !merged.is_empty() {
                            break;
                        }
                    } else {
                        if show_line_number {
                            print!("{}: ", fmt_cyan(&idx.to_string(), color));
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
