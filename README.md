USAGE:
    fundoubler.exe [FLAGS] [OPTIONS] [ARGS]

FLAGS:
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
        --sort-mod             Sort results by name
        --sort-mod-desc        Sort results by name
        --sort-name            Sort results by name
        --sort-name-desc       Sort results by name in reverse order
        --sort-size            Sort results by size
        --sort-size-desc       Sort results by size in reverse order
    -V, --version              Prints version information
    -v, --verbose              Verbose mode (-v, -vv, -vvv, etc.)

OPTIONS:
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

ARGS:
    <path-start>    start path, . if not present
    <out>           output path, stdout if not present
