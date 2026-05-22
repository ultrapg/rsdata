use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::SystemTime;
use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::Regex;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct FileTypeDef {
    pub name: String,
    pub globs: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct MatchInfo {
    pub path: String,
    pub line: Option<usize>,
    pub content: Option<String>,
    pub matches: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct FileResult {
    pub path: String,
    pub matches: usize,
    pub lines: Vec<MatchInfo>,
}

pub fn builtin_file_types() -> Vec<FileTypeDef> {
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

pub fn parse_patterns(cli: &crate::Cli) -> Vec<String> {
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

pub fn build_regex_set(patterns: &[String], cli: &crate::Cli) -> Result<Vec<Regex>, String> {
    let mut regexes = Vec::new();

    let case_override = if cli.ignore_case {
        true
    } else if cli.case_sensitive {
        false
    } else {
        !patterns.iter().any(|p| p.chars().any(|c| c.is_uppercase()))
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

pub fn build_glob_set(globs: &[String]) -> Result<(GlobSet, GlobSet), String> {
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

pub fn type_to_globs(types: &[String], include: bool) -> Vec<String> {
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

pub fn parse_size(size_str: &str) -> Result<u64, String> {
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

pub fn is_dir_excluded(name: &str, cli: &crate::Cli) -> bool {
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

pub fn file_passes_filters(path: &Path, metadata: &fs::Metadata, cli: &crate::Cli) -> bool {
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

pub fn match_content(content: &str, regexes: &[Regex], cli: &crate::Cli) -> (bool, Vec<String>) {
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

pub fn format_path(path: &Path, cli: &crate::Cli) -> String {
    if cli.absolute_path {
        if let Ok(abs) = path.canonicalize() {
            return abs.to_string_lossy().to_string();
        }
    }
    path.to_string_lossy().to_string()
}
