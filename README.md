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

Default behavior compares by **size** and **XXH3** hash. You can add or override via CLI or config file. Multiple criteria can be combined; files must match on **all** enabled criteria to be considered duplicates.

- **By name**: `fundoubler --name` or `-n`
- **By size**: `fundoubler --size`
- **By creation date**: `fundoubler --create-date`
- **By last modified date**: `fundoubler --mod-date`
- **By content (all hashes)**: `fundoubler --content`
- **By MD5**: `fundoubler --md5`
- **By SHA512**: `fundoubler --sha512`
- **By XXH3 (fast)**: `fundoubler --xxh3`
- **Combine** (e.g. size AND name, or mod-date AND MD5): `fundoubler --size --name` or `fundoubler --mod-date --md5`

### Filtering options

- **Size range (bytes)**: `fundoubler --min-size 1024 --max-size 1048576`
- **Name pattern (regex)**: `fundoubler --filter ".*\.(jpg|png)$"`
- **Limit groups shown**: `fundoubler --limit 10`
- **Hash buffer size** (bytes, default 64KB): `fundoubler --hash-buffer-size 131072`

### Deletion options

- **Interactive deletion**: `fundoubler --delete`
- **Force delete without confirmation (DANGEROUS)**: `fundoubler --delete --force-delete`
- **Dry run (no deletions)**: `fundoubler --delete --dry-run`
- **Skip confirmation prompts** (scripts, CI): `fundoubler --skip-confirm` — assumes "yes" to all prompts

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

### Output control

- **Sort order** (can repeat): `fundoubler --sort SizeDesc --sort Name`
  - Values: `Name`, `NameDesc`, `Size`, `SizeDesc`, `Created`, `CreatedDesc`, `Modified`, `ModifiedDesc`
- **Verbose**: `fundoubler -v` or `fundoubler -vv`
- **Silent**: `fundoubler --silent`

### Configuration file

Use a TOML file to set defaults; CLI options override the file.

**Create a default config file** (no need to write from scratch):

```bash
fundoubler --init-config                    # Creates fundoubler.toml in current directory
fundoubler --init-config /path/to/my.toml   # Creates config at specified path
```

**Load and use the config file:**

```bash
fundoubler --config fundoubler.toml
```

**Example structure** (run `--init-config` to generate a full template):

```toml
path_start = "."
compare_by_xxh3 = true
compare_by_size = true
compare_by_name = false
min_size = 0
max_size = 1073741824
hash_buffer_size = 65536
sort_orders = ["SizeDesc", "Name"]
dry_run = true
```

If the config path is missing or invalid, the program exits with an error.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.
