# rsdata

A lightning-fast, cross-platform recursive file search and content matching tool written in Rust. Think of it as a hybrid of `ripgrep` + `fd` + classic `grep`/`find`, designed for developers and power users.

## Features

- **Blazing fast** - Built on Rust's `walkdir`, `regex`, and `globset` crates
- **Cross-platform** - Works on Windows, Linux, and macOS
- **Content search** - Regex patterns, literal strings, word boundaries, invert matching
- **File filtering** - Extension, glob, file type, size, modification time, and more
- **Beautiful output** - Colored matches, context lines (with merged overlapping ranges), heading mode, JSON format
- **Compatible flags** - Familiar grep/ripgrep-style CLI interface

## Quick Start

```bash
# Search recursively for files containing "fn main"
rsdata --pattern "fn main"

# Use short flags and multiple patterns
rsdata -e "TODO" -e "FIXME" -t rs src/

# Filter by file type + show context
rsdata -p "error" -t rs -C 3 .

# JSON output for scripts/tools
rsdata -p "struct" -t rs --json .
```

## Installation

### From Source

```bash
cargo install --path .
```

Or simply use the binary from `./target/release/rsdata` after building.

## Usage

```
rsdata [OPTIONS] [DIRECTORY]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `[DIRECTORY]` | Directory to search in (default: `.`) |

### Pattern Matching

| Flag | Description |
|------|-------------|
| `-p, --pattern <PATTERN>` | Content pattern to search for |
| `-e, --regexp <PATTERNS>` | Additional patterns (OR logic; use multiple times) |
| `-f, --file <PATH>` | Read patterns from file, one per line |
| `-F, --fixed-strings` | Treat pattern as literal string (not regex) |
| `-w, --word-regexp` | Match whole words only |
| `-x, --line-regexp` | Match entire lines only |
| `-v, --invert-match` | Invert: show lines/files that do NOT match |
| `-s, --case-sensitive` | Case-sensitive search |
| `-i, --ignore-case` | Case-insensitive search |
| `-S, --smart-case` | Case-insensitive unless pattern contains uppercase |

> **Note:** The default mode is smart-case. Pass `-s` for strict case-sensitive or `-i` for fully case-insensitive.

### File Filtering

| Flag | Description |
|------|-------------|
| `-g, --glob <GLOBS>` | Glob patterns (use `!` prefix to exclude) |
| `-t, --type <TYPES>` | File type aliases: `rs`, `py`, `js`, `ts`, `html`, `css`, `json`, `toml`, `yaml`, `md`, `c`, `cpp`, `java`, `go`, `rb`, `php`, `sh`, `text`, etc. |
| `-T, --type-not <TYPES_NOT>` | Exclude by file type |
| `-E, --extension <EXTENSIONS>` | Filter by extension(s) |
| `--exclude-extension <EXT>` | Exclude extensions |
| `--exclude-dir <DIRS>` | Exclude directories (comma-separated) |
| `--hidden` | Include hidden files/directories |
| `-u, --no-ignore` | Don't respect `.gitignore`/`.ignore` files |
| `-L, --follow` | Follow symbolic links |
| `-d, --max-depth <N>` | Maximum recursion depth |
| `--min-size <SIZE>` | Minimum file size (suffixes: `k`, `M`, `G`, `T`) |
| `--max-size <SIZE>` | Maximum file size |
| `--changed-within <DUR>` | Modified within: `1h`, `30min`, `2d`, `1week` |
| `--changed-before <DUR>` | Modified before time window |

### Output Control

| Flag | Description |
|------|-------------|
| `-A, --after-context <N>` | Show N lines after each match |
| `-B, --before-context <N>` | Show N lines before each match |
| `-C, --context <N>` | Shortcut for `-B N -A N` |
| `-n, --line-number` | Show line numbers |
| `-l, --files-with-matches` | Only show filenames (not content) |
| `--files-without-match` | Invert: show files without match |
| `-c, --count` | Show count of matches per file |
| `-o, --only-matching` | Show only matched text, not full lines |
| `-H, --with-filename` | Always prefix with filename |
| `--no-filename` | Never prefix with filename |
| `--heading` | Group matches under file headings with underline |
| `--passthru` | Print ALL lines, highlighting matches |
| `-a, --text` | Treat binary files as text |
| `--json` | JSON output (for scripts/editor integration) |
| `--stats` | Print statistics summary |
| `-q, --quiet` | Quiet mode: exit code only |
| `-0, --null` | NUL-separated output (for `xargs -0`) |
| `--absolute-path` | Show absolute paths |
| `--files` | Only list files, don't search content |
| `-m, --max-count <N>` | Stop after N matches per file |
| `--color <when>` | Color mode: `auto` (default), `always`, `never` |

> **Context lines:** When matches are close together, overlapping context ranges are automatically merged into a single block. Non-overlapping blocks are separated by `--`.

## Examples

### Content Search

```bash
# Basic regex search
rsdata -p "Result<.*, Error>" -t rs .

# Multiple patterns (OR)
rsdata -e "unwrap()" -e "expect(" -t rs .

# Inverted match (files without pattern)
rsdata -p "unsafe" -v --files-without-match -t rs .

# Word boundaries only
rsdata -p "result" -w .
```

### File Filtering

```bash
# By extension
rsdata --files -E rs,toml,json --exclude-dir target,.git .

# By glob
rsdata --files -g '*.rs' -g '!*test*' .

# By file type alias
rsdata -p "console.log" -t js -T min .

# By size
rsdata --files --min-size 1M --max-size 100M .

# By modification time
rsdata --files --changed-within 24h .
rsdata --files --changed-before 1week .
```

### Context & Output

```bash
# Show 2 lines before and after (merged if overlapping)
rsdata -p "fn main" -C 2 .

# Just filenames (for scripting)
rsdata -p "TODO" -l . | xargs code

# JSON for tools
rsdata -p "struct" --json -t rs src/

# Stats
rsdata -p "let" --stats -t rs .

# Disable colors (pipe-safe)
rsdata -p "error" --color never .
```

## Cross-Platform Compilation

rsdata works on Windows, Linux, and macOS. To cross-compile:

```bash
# For Windows (from Linux)
cargo build --release --target x86_64-pc-windows-gnu

# For macOS (requires cross-compiler toolchain)
cargo build --release --target x86_64-apple-darwin
```

Or build natively on each platform.

## License

GNU General Public License v3.0

## Contributing

Contributions are welcome! Please feel free to submit a PR.
