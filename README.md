## Fundoubler

A fast, cross-platform utility for finding and removing file duplicates written in Rust.

## Features

- **Fast scanning**: Uses parallel processing for large directories
- **Multiple comparison methods**: Size, name, dates, and various hash algorithms (MD5, SHA512, XXH3)
- **Smart filtering**: Filter by size or name patterns (regex)
- **Safe deletion**: Interactive confirmation or dry-run mode
- **Progress reporting**: Visual progress bars for long operations
- **Export results**: Save duplicate reports to files
- **Configuration file**: TOML config with `--config`; CLI overrides file options
- **Cross-platform**: Works on Windows, macOS, and Linux

## Installation

Build from source:

```bash
git clone https://github.com/AATruttse/fundoubler
cd fundoubler
cargo build --release
```

## Usage

```text
fundoubler [OPTIONS] [PATH_START] [OUTPUT]
```

- **PATH_START**: Directory to scan (default: current directory `.`)
- **OUTPUT**: Optional file to write results to (default: stdout)

Run `fundoubler --help` for the full list of options.

### Basic usage

- **Find duplicates in current directory**: `fundoubler`
- **Find duplicates in a directory**: `fundoubler /path/to/directory`
- **Find duplicates by content (enables MD5, SHA512, XXH3)**: `fundoubler --content /path/to/directory`
- **Save results to file**: `fundoubler /path/to/directory duplicates.txt`

### Comparison options (CLI)

Default behavior compares by **size** and **XXH3** hash. You can override via CLI or config file.

- **By content (all hashes)**: `fundoubler --content`
- **By MD5**: `fundoubler --md5`
- **By SHA512**: `fundoubler --sha512`
- **By XXH3 (fast)**: `fundoubler --xxh3`
- **Combine**: `fundoubler --content --md5`

Comparison by **name**, **created date**, or **modified date** is supported only via the config file (see below).

### Filtering options

- **Size range (bytes)**: `fundoubler --min-size 1024 --max-size 1048576`
- **Name pattern (regex)**: `fundoubler --filter ".*\.(jpg|png)$"`
- **Limit groups shown**: `fundoubler --limit 10`

### Deletion options

- **Interactive deletion**: `fundoubler --delete`
- **Force delete without confirmation (DANGEROUS)**: `fundoubler --delete --force-delete`
- **Dry run (no deletions)**: `fundoubler --delete --dry-run`

### Output control

- **Sort order** (can repeat): `fundoubler --sort SizeDesc --sort Name`
  - Values: `Name`, `NameDesc`, `Size`, `SizeDesc`, `Created`, `CreatedDesc`, `Modified`, `ModifiedDesc`
- **Verbose**: `fundoubler -v` or `fundoubler -vv`
- **Silent**: `fundoubler --silent`

### Configuration file

Use a TOML file to set defaults; CLI options override the file.

**Example `fundoubler.toml`:**

```toml
path_start = "."

# Comparison (at least one must be true)
compare_by_xxh3 = true
compare_by_size = true
compare_by_name = false
compare_by_created = false
compare_by_modified = false
compare_by_md5 = false
compare_by_sha512 = false

# Filters
min_size = 1024
max_size = 1073741824
name_filter = ".*\\.(jpg|png)$"

# Output
sort_orders = ["SizeDesc", "Name"]
limit = 100
verbose = 0
silent = false
dry_run = true

# Deletion
delete = false
force_delete = false
```

**Use the config file:**

```bash
fundoubler --config fundoubler.toml
```

If the config path is missing or invalid, the program exits with an error.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.
