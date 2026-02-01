## Fundoubler

A fast, cross-platform utility for finding and removing file duplicates written in Rust.

## Features

- **Fast scanning**: Uses parallel processing for large directories
- **Multiple comparison methods**: Size, name, creation/modification dates, and hash algorithms (MD5, SHA512, XXH3)
- **Smart filtering**: Filter by size, name patterns (regex), exclude directories
- **Source directories**: Prefer files in specified dirs when choosing which duplicate to keep
- **Safe deletion**: Interactive per-file confirmation, dry-run mode, or unattended with `--skip-confirm`
- **Delete log and restore**: Records deletions for undo; restore files from logs with `--restore`
- **Hash cache**: Avoid re-hashing on re-scans (optional)
- **Logging**: Configurable log levels to file
- **Progress reporting**: Visual progress bars for long operations
- **Export results**: Save duplicate reports to files
- **Configuration file**: TOML config with `--config`; CLI options override file settings
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
- **OUTPUT**: Optional file to write results to (positional, after path)

**Modes** (mutually exclusive):
- **Normal scan**: Find duplicates in `PATH_START`, optionally delete, write report
- **Restore**: `fundoubler --restore` — restore files from delete log (skips scan)
- **Init config**: `fundoubler --init-config` — create default config file and exit

Run `fundoubler --help` for the full list of options.

### Basic usage

- **Find duplicates in current directory**: `fundoubler`
- **Find duplicates in a directory**: `fundoubler /path/to/directory`
- **Find duplicates by content** (MD5, SHA512, XXH3): `fundoubler --content` or `fundoubler -t`
- **Save results to file**: `fundoubler /path/to/directory report.txt`

### Comparison options (CLI)

Default behavior compares by **size** and **XXH3** hash. Via CLI or config file:

- **If you pass any comparison flag** (`--name`, `--size`, `--md5`, etc.), **only** those criteria are used (e.g. `--size` alone = no hashing, fast).
- **If you pass none**, config/default values apply; additional flags add to them.
- Multiple criteria can be combined; files must match on **all** enabled criteria to be considered duplicates.

- **By name**: `fundoubler --name` or `-n`
- **By size**: `fundoubler --size`
- **By creation date**: `fundoubler --create-date`
- **By last modified date**: `fundoubler --mod-date`
- **By content (all hashes)**: `fundoubler --content` or `-t`
- **By MD5**: `fundoubler --md5`
- **By SHA512**: `fundoubler --sha512`
- **By XXH3 (fast)**: `fundoubler --xxh3`
- **Combine** (e.g. size AND name, or mod-date AND MD5): `fundoubler --size --name` or `fundoubler --mod-date --md5`

### Filtering options

- **Size range (bytes)**: `fundoubler --min-size 1024 --max-size 1048576`
- **Name pattern (regex)**: `fundoubler --filter ".*\.(jpg|png)$"`
- **Exclude directories** (repeatable): `fundoubler --exclude-dir node_modules --exclude-dir target`
- **Source directories** (repeatable): `fundoubler --source-dir /backup/primary` — when duplicates are found, files in source dirs are kept; others are marked for deletion
- **Limit groups shown**: `fundoubler --limit 10`

### Hash options
- **Hash buffer size** (bytes, default 64KB): `fundoubler --hash-buffer-size 131072`
- **Hash cache** (default: off): `fundoubler --hash-cache` — avoid re-hashing on re-scans
- **Hash cache directory** (default: `.fundoubler/.hashcache`): `fundoubler --hash-cache-dir /path/to/cache`

### Deletion options

- **Interactive deletion**: `fundoubler --delete` or `-d`
- **Force delete without confirmation (DANGEROUS)**: `fundoubler --delete --force-delete` or `-f`
- **Dry run (no deletions)**: `fundoubler --delete --dry-run`
- **Skip confirmation prompts** (scripts, CI): `fundoubler --skip-confirm` — assumes "yes" to all prompts
- **Delete log** (default: on): Records each deleted file and its kept duplicate for restore. Use `--no-delete-log` to disable.
- **Restore**: `fundoubler --restore` uses the latest delete log; `fundoubler --restore /path/to/log` uses a specific file. Prompts for each file unless `--skip-confirm`. Use `--logs-dir` if logs are not in `./logs`.


### Deletion flag interaction

How `--delete`, `--force-delete`, `--dry-run`, and `--skip-confirm` work together:

| `--delete` | `--force-delete` | `--dry-run` | `--skip-confirm` | Result |
|:----------:|:----------------:|:-----------:|:----------------:|--------|
| ❌ | - | - | - | No deletion (scan only) |
| ✅ | ❌ | ✅ | - | Dry run: show what would be deleted, no actual deletions |
| ✅ | ❌ | ❌ | ❌ | Interactive: prompt for each file to delete |
| ✅ | ❌ | ❌ | ✅ | Delete all duplicates (assume yes to each prompt) |
| ✅ | ✅ | ✅ | - | Dry run (no deletions; `--dry-run` wins) |
| ✅ | ✅ | ❌ | ❌ | One global "Are you sure?" prompt, then delete all |
| ✅ | ✅ | ❌ | ✅ | Delete all duplicates with no prompts |

**Important:**

- **`--force-delete`** has no effect without `--delete`. Deletion logic only runs when `--delete` is set.
- **`--dry-run` and `--skip-confirm` together** never delete anything. `--dry-run` always blocks deletion, regardless of other flags.

**Flow:**

1. **`--dry-run`** — Always blocks deletion. Use it to preview changes safely.
2. **`--force-delete`** (requires `--delete`) — Skips per-file prompts; delete all duplicates at once. Shows one global confirmation unless `--skip-confirm` is set.
3. **`--skip-confirm`** — Assumes "yes" to all prompts (both global and per-file). Use in scripts or CI. Does nothing if `--dry-run` is set.

**Examples for scripts:**
```bash
fundoubler --delete --dry-run /path              # Preview only
fundoubler --delete --force-delete --skip-confirm /path   # Unattended deletion
```

**Delete log and restore:**
- Delete logs: `logs_dir/del_logs/YYYYMMDDHHMMSSfundel.log` (default `./logs/del_logs/`)
- Each record: deleted path + kept duplicate path
- `fundoubler --restore` — use latest log (in `./logs/del_logs/` or `--logs-dir`)
- `fundoubler --restore /path/to/20260124120000fundel.log` — use specific log
- Prompts for each file unless `--skip-confirm`

### Logging

- **Logging level** (repeatable, like verbose): `fundoubler -l` (errors), `-ll` (+info), `-lll` (+debug)
- **Logs directory** (default: `./logs`): `fundoubler --logs-dir /var/log/fundoubler`
- Log files: `YYYYMMDDHHMMSSfun.log` (e.g. `20260124003905fun.log`)
- Level 0 = off (default), 1 = errors only, 2 = errors + info, 3+ = errors + info + debug

### Output control

- **Sort order** (repeatable): `fundoubler --sort size-desc --sort name`
  - CLI values (kebab-case): `name`, `name-desc`, `size`, `size-desc`, `created`, `created-desc`, `modified`, `modified-desc`
- **Verbose**: `fundoubler -v` or `fundoubler -vv` (shows wasted space; `-vv` shows config)
- **Silent**: `fundoubler -s` or `fundoubler --silent` (no console output)

### Configuration file

Use a TOML file to set defaults; CLI options override the file.

**Create a default config file** (no need to write from scratch):

```bash
fundoubler --init-config                    # Creates fundoubler.toml in current directory
fundoubler --init-config /path/to/my.toml   # Creates config at specified path
fundoubler --init-config --silent           # Suppress confirmation message
```

**Load and use the config file:**

```bash
fundoubler --config fundoubler.toml
```

**Example structure** (run `--init-config` to generate a full template):

```toml
path_start = "."
compare_by_size = true
compare_by_xxh3 = true
compare_by_name = false
compare_by_created = false
compare_by_modified = false
compare_by_md5 = false
compare_by_sha512 = false
min_size = 0
max_size = 1073741824
exclude_dirs = ["node_modules", "target", ".git"]
source_dirs = ["./backup", "/primary/photos"]
hash_buffer_size = 65536
hash_cache = false
hash_cache_dir = ".fundoubler/.hashcache"
log_level = 0
logs_dir = "./logs"
delete_log = true
sort_orders = ["SizeDesc", "Name"]
verbose = 0
silent = false
```

**Key options:**
- **name_filter**: Regex to match filenames (e.g. `".*\\.(jpg|png)$"`). Omit to include all.
- **exclude_dirs**: Directories to skip during scan. Paths relative to `path_start` or absolute.
- **source_dirs**: When duplicates are found, files in these dirs are kept; others marked for deletion.
- **log_level**: 0 = off, 1 = error, 2 = info, 3 = debug. Logs go to `logs_dir`.
- **logs_dir**: Directory for log files (default `./logs`). Files: `YYYYMMDDHHMMSSfun.log`.
- **delete_log**: If true (default), record deletions in `logs_dir/del_logs/` for `--restore`.
- **sort_orders**: In config use PascalCase (`SizeDesc`, `Name`); in CLI use kebab-case (`size-desc`, `name`).

**Note:** `delete`, `force_delete`, etc. from config are overwritten by CLI. Use CLI flags to trigger deletion.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.
