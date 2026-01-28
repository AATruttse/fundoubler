## Fundoubler

A fast, cross-platform utility for finding and removing file duplicates written in Rust.

## Featuress

- **Fast scanning**: Uses parallel processing for large directories
- **Multiple comparison methods**: Size, name, dates, and various hash algorithms
- **Smart filtering**: Filter by size, date, or name patterns
- **Safe deletion**: Interactive confirmation or dry-run mode
- **Progress reporting**: Visual progress bars for long operations
- **Export results**: Save duplicate reports to files
- **Cross-platform**: Works on Windows, macOS, and Linux


## USAGE:
    fundoubler.exe [FLAGS] [OPTIONS] [ARGS]

# Basic usage

**Find duplicates in current directory**: fundoubler
**Find duplicates in specific directory**: fundoubler /path/to/directory
**Find duplicates by content (uses fast xxh3 hash by default)**: fundoubler --content /path/to/directory
**Find duplicates and save results to file**: fundoubler /path/to/directory duplicates.txt

# Comparison options

**Compare by file size (default)**: fundoubler --size
**Compare by MD5 hash**: fundoubler --md5
**Compare by SHA512 hash (more secure, slower)**: fundoubler --sha512
**Compare by XXH3 hash (fast, recommended for large files)**: fundoubler --xxh3
**Combine multiple criteria**: fundoubler --size --name --md5

# Filtering options

**Filter by size range**: fundoubler --min-size 1024 --max-size 1048576
**Filter by name pattern (regex)**: fundoubler --filter ".*\.(jpg|png)$"
**Limit number of results**: fundoubler --limit 10

# Deletion options

**Interactive deletion (safe)**: fundoubler --delete
**Force delete without confirmation (DANGEROUS!)**: fundoubler --delete --force-delete
**Dry run - see what would be deleted**: fundoubler --delete --dry-run

# Output control

**Sort results**: fundoubler --sort size --sort name
**Verbose output**: fundoubler -vvv
**Silent mode (no output)**: fundoubler --silent

## FLAGS:
    -t, --content              Check files by content
    -c, --date-created         Check files by datetime of creation
    -m, --date-modified        Check files by datetime of modification
        --debug                Debug
        --debug-config         Show config options
    -d, --delete               Delete unneeded doubles. Be careful!
    -f, --force-delete         Force delete unneeded doubles. Be very careful!
    -h, --hash                 Check files by MD5 and SHA512 hashes
        --md5                  Check files by MD5 hash
        --sha512               Check files by SHA512 hash
        --help                 Prints help information
        --hide-config          Hides config from debug show. Useful only .cfg file
    -n, --name                 Check files by size
        --show-options-only    Show options only - no real work
    -S, --silent               Silent mode
    -s, --size                 Check files by size
        --sort-create          Sort results by create date
        --sort-create-desc     Sort results by create date in reverse order
        --sort-mod             Sort results by mod date
        --sort-mod-desc        Sort results by mod date in reverse order
        --sort-name            Sort results by name
        --sort-name-desc       Sort results by name in reverse order
        --sort-size            Sort results by size
        --sort-size-desc       Sort results by size in reverse order
    -V, --version              Prints version information
    -v, --verbose              Verbose mode (-v, -vv, -vvv, etc.)

## OPTIONS:
        --defaults-file <configfile>          File with defaults config [default: ]
    -F, --first-n <first-n>                   First n files with maximum doubles to show [default: 0]
    -l, --log <log>                           Log file [default: ]
        --max-create-date <max-createdate>    Maximum create date of files to be checked [default: ]
        --max-mod-date <max-moddate>          Maximum modify of files to be checked [default: ]
        --max-size <max-size>                 Maximum size of files to be checked [default: 0]
        --min-create-date <min-createdate>    Minimum create date of files to be checked [default: ]
        --min-mod-date <min-moddate>          Minimum modify date of files to be checked [default: ]
        --min-size <min-size>                 Minimum size of files to be checked [default: 0]
        --name-filter <name-filter>           File names filter [default: ]

## ARGS:
    <path-start>    start path, . if not present
    <out>           output path, stdout if not present

## Configuration

Fundoubler supports configuration files. Create fundoubler.toml

# Configuration file

**Default comparison method**
compare_by_xxh3 = true
compare_by_size = true

**Default filters**
min_size = 1024  # 1KB minimum
max_size = 1073741824  # 1GB maximum

**Default sort order**
sort_orders = ["size", "name"]

**Always use dry-run mode for safety**
dry_run = true

# Use the config file:

fundoubler --config fundoubler.toml

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.