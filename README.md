## Fundoubler

A fast, cross-platform utility for finding and removing file duplicates written in Rust.

## Features

- **Fast scanning**: Uses parallel processing for large directories
- **Multiple comparison methods**: Size, name, creation/modification dates, and hash algorithms (MD5, SHA512, XXH3)
- **Smart filtering**: Filter by size, name patterns (regex), time ranges (creation/modification), user and group (Unix), exclude directories
- **Source directories**: Prefer files in specified dirs when choosing which duplicate to keep
- **Search directories**: Only report duplicate groups that have at least one file in specified dirs (e.g. find duplicates of source files in a specific folder)
- **Safe deletion**: Interactive per-file confirmation, dry-run mode, or unattended with `--skip-confirm`
- **Link creation**: Replace duplicates with symlinks, hardlinks, or Windows shortcuts instead of deleting
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

Run tests: `cargo test`. Run benchmarks: `cargo bench`.

## Usage

```text
fundoubler [OPTIONS] [PATH_START] [OUTPUT]
```

- **PATH_START**: Directory to scan (default: current directory `.`)
- **OUTPUT**: Optional file to write duplicate report to (positional, after PATH_START). Omit to print to stdout.

**Modes** (mutually exclusive):
- **Normal scan**: Find duplicates in `PATH_START`, optionally delete, write report
- **Restore**: `fundoubler --restore` — restore files from delete log (skips scan)
- **Init config**: `fundoubler --init-config` — create default config file and exit

Run `fundoubler --help` for the full list of options.

**Short flags:** `-n` (name), `-d` (delete), `-f` (force-delete), `-t` (content), `-v`/`-vv` (verbose), `-s` (silent), `-l`/`-ll`/`-lll` (log level).

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
- **By content (all hashes)**: `fundoubler --content` or `-t` — enables MD5, SHA512, and XXH3
- **By MD5**: `fundoubler --md5`
- **By SHA512**: `fundoubler --sha512`
- **By XXH3 (fast)**: `fundoubler --xxh3`
- **Combine** (e.g. size AND name, or mod-date AND MD5): `fundoubler --size --name` or `fundoubler --mod-date --md5`

**Note:** When `--content` is combined with other comparison flags, it adds all three hash algorithms to the specified criteria. For example, `--content --size` compares by size AND all three hashes.

### Filtering options

- **Size range (bytes)**: `fundoubler --min-size 1024 --max-size 1048576`
- **Name pattern (regex)**: `fundoubler --filter ".*\.(jpg|png)$"`
- **Creation time range**: `fundoubler --min-create-time 2024-01-01 --max-create-time 2024-12-31`
- **Modification time range**: `fundoubler --min-mod-time 2024-01-01 --max-mod-time 2024-12-31`

  Time format: `YYYY-MM-DD`, `YYYY-MM-DD HH:MM:SS`, `YYYY-MM-DDTHH:MM:SS`, RFC 3339, or Unix timestamp. Works on Windows and Linux.

- **User filter** (Unix only; ignored on Windows): `fundoubler --user-filter myuser` or `--user-filter 1000` — only files owned by this user
- **Group filter** (Unix only; ignored on Windows): `fundoubler --group-filter mygroup` or `--group-filter 100` — only files in this group
- **Exclude directories** (repeatable): `fundoubler --exclude-dir node_modules --exclude-dir target`
- **Source directories** (repeatable): `fundoubler --source-dir /backup/primary` — when duplicates are found, files in source dirs are kept; others are marked for deletion
- **Search directories** (repeatable): `fundoubler --search-dir /path/to/search` — only report duplicate groups that have at least one file in these directories. Use with `--source-dir` to find duplicates of source files that lie in the search dirs. When not set, all duplicate groups under the scan root are reported.
- **Unique** (requires `--search-dir` and `--source-dir`): `fundoubler --unique` — show only files that are *not* duplicates of origin (source dir). Excludes groups that have any file in source dir; reports only groups where all files are outside origin. With `--delete`, deletes duplicates (or creates links) in the reported groups as usual.
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
- **Restore**: `fundoubler --restore` uses the latest delete log; `fundoubler --restore /path/to/log` uses a specific file. Prompts for each file unless `--skip-confirm`. Use `--logs-dir /path` if logs are not in `./logs`; with `--restore`, delete logs are read from `/path/del_logs/`.

  **Restore behavior:**
  - Restores files by copying from the kept duplicate recorded in the log
  - Skips files that already exist at the target location
  - Reports errors if the source file (kept duplicate) is missing
  - Creates parent directories as needed
  - Per-file confirmation unless `--skip-confirm` is used

### Link creation options (requires --delete)

Instead of deleting duplicate files, create links pointing to the kept duplicate:

- **Create symlinks**: `fundoubler --delete --create-symlinks` — replace duplicates with symlinks (Unix/Windows)
- **Create hardlinks**: `fundoubler --delete --create-hardlinks` — replace duplicates with hardlinks (Unix/Windows)
- **Create Windows shortcuts**: `fundoubler --delete --create-shortcuts` — replace duplicates with .lnk shortcuts (Windows only)
- **Use kept file's name for links**: `fundoubler --delete --create-symlinks --no-keep-link-names` — links use the kept file's name instead of deleted file's name (shortcuts always get .lnk extension)

**Behavior:**
- Without `--no-keep-link-names`: Symlinks/hardlinks keep the deleted file's name; shortcuts get deleted file's name + `.lnk`
- With `--no-keep-link-names`: Symlinks/hardlinks use the kept file's name; shortcuts use kept file's name + `.lnk`
- Only one link type can be specified at a time
- Link options require `--delete`; without `--delete` they are ignored
- Confirmation dialogs and dry-run output show which link would be created
- Original file is deleted first, then link is created at the same location
- On Windows, symlinks may require admin privileges or developer mode; shortcuts work without special permissions


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
fundoubler --delete --create-symlinks /path      # Replace duplicates with symlinks
fundoubler --delete --create-shortcuts --skip-confirm /path  # Replace with shortcuts (Windows)
```

**Delete log and restore:**
- Delete logs: `logs_dir/del_logs/YYYYMMDDHHMMSSfundel.log` (default `./logs/del_logs/`)
- Each record: two lines per pair — `deleted:<path>` then `source:<path>` (the kept duplicate)
- `fundoubler --restore` — use latest log (in `./logs/del_logs/` or `--logs-dir/del_logs/`)
- `fundoubler --restore /path/to/20260124120000fundel.log` — use specific log file
- Prompts for each file unless `--skip-confirm`
- If a deleted file already exists, restore skips it
- If the kept duplicate (source) is missing, restore reports an error and continues with other files

### Logging

- **Logging level** (repeatable, like verbose): `fundoubler -l` (errors), `-ll` (+info), `-lll` (+debug)
- **Logs directory** (default: `./logs`): `fundoubler --logs-dir /var/log/fundoubler`
- Log files: `YYYYMMDDHHMMSSfun.log` (e.g. `20260124003905fun.log`)
- Level 0 = off (default), 1 = errors only, 2 = errors + info, 3+ = errors + info + debug

### Output control

- **Sort order** (repeatable): `fundoubler --sort size-desc --sort name`
  - CLI values (kebab-case): `name`, `name-desc`, `size`, `size-desc`, `created`, `created-desc`, `modified`, `modified-desc`
  - Multiple sort orders create a multi-level sort (primary, secondary, etc.)
  - Default: `size-desc` then `name` (largest files first, then alphabetical)
  - Sort order determines which file is kept in each duplicate group (first file after sorting is kept)
- **Verbose**: `fundoubler -v` or `fundoubler -vv` (shows wasted space; `-vv` shows config)
- **Silent**: `fundoubler -s` or `fundoubler --silent` (no console output; also disables progress bar)
- **No progress bar**: `fundoubler --no-progress-bar` — hide the scan progress bar (scripts, CI)

The progress bar shows file count (e.g. `[=>---] 50/100 Scanning files...`) during the scan. It appears on stderr and is visible only when running in an interactive terminal (not when piping output).

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
max_size = 1073741824  # Example: 1GB limit (default is u64::MAX = no limit)
# name_filter = ".*\\.(jpg|png)$"   # Regex to match filenames
# min_create_time = "2024-01-01"
# max_create_time = "2024-12-31"
# min_mod_time = "2024-02-01"
# max_mod_time = "2024-11-30"
# user_filter = "myuser"   # Unix only; ignored on Windows
# group_filter = "mygroup" # Unix only; ignored on Windows
exclude_dirs = ["node_modules", "target", ".git"]
source_dirs = ["./backup", "/primary/photos"]
# search_dirs = ["./downloads", "./cache"]   # Only report groups that have a file in these dirs
hash_buffer_size = 65536
hash_cache = false
hash_cache_dir = ".fundoubler/.hashcache"
log_level = 0
logs_dir = "./logs"
delete_log = true
# create_symlinks = false
# create_hardlinks = false
# create_shortcuts = false   # Windows only
# no_keep_link_names = false
sort_orders = ["SizeDesc", "Name"]
verbose = 0
silent = false
no_progress_bar = false
```

**Key options:**
- **path_start**: Directory to scan (default: `.`). CLI: positional argument.
- **output**: File to write duplicate report to (default: stdout). CLI: second positional argument.
- **compare_by_***: Comparison criteria flags. Default: `compare_by_size = true`, `compare_by_xxh3 = true`.
- **name_filter**: Regex to match filenames (e.g. `".*\\.(jpg|png)$"`). Omit to include all. CLI: `--filter`.
- **min_size, max_size**: File size range in bytes. Default: `min_size = 0`, `max_size = 18446744073709551615` (u64::MAX).
- **min_create_time, max_create_time**: Only include files created within this range.
- **min_mod_time, max_mod_time**: Only include files modified within this range.

  Time formats: `YYYY-MM-DD`, `YYYY-MM-DD HH:MM:SS`, `YYYY-MM-DDTHH:MM:SS`, RFC 3339, Unix timestamp. On some Linux filesystems creation time may be unavailable; those files are then included.

- **user_filter, group_filter**: (Unix only; ignored on Windows) Only include files owned by this user / in this group. Use username or numeric uid/gid.
- **exclude_dirs**: Directories to skip during scan. Paths relative to `path_start` or absolute. CLI: `--exclude-dir` (repeatable).
- **source_dirs**: When duplicates are found, files in these dirs are kept; others marked for deletion. CLI: `--source-dir` (repeatable).
- **search_dirs**: Only report duplicate groups that have at least one file in these directories. When empty (default), all duplicate groups are reported. Use with `source_dirs` to find duplicates of source files that lie in the search dirs. CLI: `--search-dir` (repeatable).
- **unique**: With `search_dirs` and `source_dirs`, show only groups where no file is in `source_dirs` (files unique to search area). Requires both `search_dirs` and `source_dirs`. CLI: `--unique`.
- **hash_buffer_size**: Buffer size for reading files during hashing (default: 65536 bytes = 64KB). CLI: `--hash-buffer-size`.
- **hash_cache**: Enable hash caching to avoid re-hashing unchanged files (default: false). CLI: `--hash-cache`.
- **hash_cache_dir**: Directory for hash cache files (default: `.fundoubler/.hashcache`). CLI: `--hash-cache-dir`.
- **log_level**: 0 = off, 1 = error, 2 = info, 3 = debug. Logs go to `logs_dir`. CLI: `-l`, `-ll`, `-lll`.
- **logs_dir**: Directory for log files (default `./logs`). Files: `YYYYMMDDHHMMSSfun.log`. CLI: `--logs-dir`.
- **delete_log**: If true (default), record deletions in `logs_dir/del_logs/` for `--restore`. CLI: `--no-delete-log` to disable.
- **sort_orders**: In config use PascalCase (`SizeDesc`, `Name`); in CLI use kebab-case (`size-desc`, `name`). Default: `["SizeDesc", "Name"]`.
- **limit**: Maximum number of duplicate groups to display (e.g. `limit = 10`). CLI: `--limit`.
- **verbose**: Verbosity level 0-2 (default: 0). Level 1 shows wasted space, level 2 shows config. CLI: `-v`, `-vv`.
- **silent**: If true, suppress all console output and disable progress bar (default: false). CLI: `-s`, `--silent`.
- **no_progress_bar**: If true, hide the scan progress bar (default: false). CLI: `--no-progress-bar`.
- **create_symlinks, create_hardlinks, create_shortcuts**: Link creation options (require `delete = true`). Only one can be true at a time. CLI: `--create-symlinks`, `--create-hardlinks`, `--create-shortcuts`.
- **no_keep_link_names**: If true, links use kept file's name instead of deleted file's name (shortcuts always get .lnk). CLI: `--no-keep-link-names`.

**Important notes:**
- **`delete`, `force_delete`, and `dry_run`** from config are **always overwritten by CLI**. Use `--delete`, `--force-delete`, or `--dry-run` to control deletion behavior.
- **Link creation options** also require `--delete` CLI flag to work (even if `delete = true` in config).
- **`skip_confirm`** is not saved in config file (CLI-only option for scripts).
- When **no comparison flags** are passed via CLI, config/default values apply. When **any comparison flag** is passed, only those flags are used (exclusive mode).

## Examples

**Find duplicates by size only (fast, no hashing):**
```bash
fundoubler --size /path/to/scan
```

**Find duplicates by content hash and delete interactively:**
```bash
fundoubler --content --delete /path/to/scan
```

**Find duplicates, replace with symlinks, skip confirmations:**
```bash
fundoubler --md5 --delete --create-symlinks --skip-confirm /path/to/scan
```

**Preview deletions without actually deleting:**
```bash
fundoubler --delete --dry-run -vv /path/to/scan
```

**Find duplicates in photos directory, exclude thumbnails:**
```bash
fundoubler --content --exclude-dir thumbnails --exclude-dir .thumbnails /photos
```

**Find duplicates of files in source dir that appear in search dir (only report groups touching search dir):**
```bash
fundoubler --content --source-dir /canonical --search-dir /downloads /path
```

**Show only files in search dir that are NOT duplicates of origin (unique files); with --delete, remove duplicates:**
```bash
fundoubler --content --source-dir /canonical --search-dir /downloads --unique /path
fundoubler --content --source-dir /canonical --search-dir /downloads --unique --delete --dry-run /path
```

**Restore deleted files from latest log:**
```bash
fundoubler --restore
```

**Use config file with CLI overrides:**
```bash
fundoubler --config myconfig.toml --delete --dry-run
```

## Troubleshooting

**Progress bar not visible:**
- Progress bar only appears in interactive terminals (TTY)
- Use `--no-progress-bar` to explicitly disable it
- Silent mode (`-s`) automatically disables progress bar

**Symlinks fail on Windows:**
- Windows symlinks require administrator privileges or developer mode
- Use `--create-shortcuts` instead for Windows (works without special permissions)

**Hash cache not working:**
- Ensure `hash_cache_dir` is writable
- Cache is keyed by file path and modification time
- Delete cache directory to force re-hashing

**Restore fails:**
- Check that the kept duplicate file still exists
- Verify log file path is correct
- Use `--logs-dir` if logs are in a non-default location

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.
