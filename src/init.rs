extern crate confy;

// #[macro_use]
// extern crate serde_derive;

use std::path::PathBuf;
//use std::str::FromStr;
use std::time::SystemTime;

use chrono::{Datelike, Utc};
use confy::load_path;
use humantime::Timestamp;
use serde_derive::{Deserialize, Serialize};
use simple_log::LogConfigBuilder;
use structopt::StructOpt;

const DEFAULT_START: &str = ".";

const DEFAULT_CFG: &str = "fundoubler.cfg";
const DEFAULT_OUT: &str = ".fundoubler%DATE%.res";
const DEFAULT_LOG: &str = "./fundoubler%DATE%.log";
const DATE_TEMPLATE: &str = "%DATE%";

const DEFAULT_FIRST_N: usize = 100;

/*#[serde_as]*/
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigFile {
    pub global_verbose: u8,
    pub hide_config: bool,
    pub debug: bool,
    pub debug_config: bool,
    pub show_options_only: bool,

    pub delete: bool,
    pub force_delete: bool,
    pub silent_mode: bool,

    pub size: bool,
    pub name: bool,
    pub date_created: bool,
    pub date_modified: bool,
    pub hash_md5: bool,
    pub hash_sha512: bool,
    pub content: bool,

    pub min_size: u64,
    pub max_size: u64,

    pub min_createdate: String,
    pub max_createdate: String,
    pub min_moddate: String,
    pub max_moddate: String,

    pub name_filter: String,

    pub first_n: usize,

    pub sort_res_name_asc: bool,
    pub sort_res_name_desc: bool,
    pub sort_res_size_asc: bool,
    pub sort_res_size_desc: bool,
    pub sort_res_cdate_asc: bool,
    pub sort_res_cdate_desc: bool,
    pub sort_res_mdate_asc: bool,
    pub sort_res_mdate_desc: bool,

    pub path_start: Option<PathBuf>,
    pub out_filename: Option<PathBuf>,
    pub log_filename: String,
}

impl Default for ConfigFile {
    fn default() -> Self {
        ConfigFile {
            global_verbose: 0,
            hide_config: false,
            debug: false,
            debug_config: false,
            show_options_only: false,

            delete: false,
            force_delete: false,
            silent_mode: false,

            size: false,
            name: false,
            date_created: false,
            date_modified: false,
            hash_md5: false,
            hash_sha512: false,
            content: false,

            min_size: 0,
            max_size: 0,

            min_createdate: "".to_string(),
            max_createdate: "".to_string(),
            min_moddate: "".to_string(),
            max_moddate: "".to_string(),

            name_filter: "".to_string(),

            first_n: DEFAULT_FIRST_N,

            sort_res_name_asc: false,
            sort_res_name_desc: false,
            sort_res_size_asc: false,
            sort_res_size_desc: false,
            sort_res_cdate_asc: false,
            sort_res_cdate_desc: false,
            sort_res_mdate_asc: false,
            sort_res_mdate_desc: false,            

            path_start: Some(PathBuf::from(DEFAULT_START)),
            out_filename: Some(PathBuf::from(DEFAULT_OUT)),
            log_filename: DEFAULT_LOG.to_string(),
        }
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "spamer")]
pub struct Options {
    // The number of occurrences of the `v/verbose` flag
    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[structopt(short, long, parse(from_occurrences))]
    pub verbose: u8,

    /// Hides config from debug show. Useful only .cfg file
    #[structopt(long = "hide-config")]
    pub hide_config: bool,

    /// Show config options
    #[structopt(long = "debug-config")]
    pub debug_config: bool,

    /// Debug
    #[structopt(long = "debug")]
    pub debug: bool,

    /// Show options only - no real work
    #[structopt(long = "show-options-only")]
    pub show_options_only: bool,

    /// File with defaults config
    #[structopt(long = "defaults-file", default_value = "")]
    pub configfile: PathBuf,

    /// Delete unneeded doubles. Be careful!
    #[structopt(short = "d", long = "delete")]
    pub delete: bool,

    /// Force delete unneeded doubles. Be very careful!
    #[structopt(short = "f", long = "force-delete")]
    pub force_delete: bool,

    /// Silent mode
    #[structopt(short = "S", long = "silent")]
    pub silent_mode: bool,

    /// Check files by size
    #[structopt(short = "s", long)]
    pub size: bool,

    /// Check files by size
    #[structopt(short = "n", long)]
    pub name: bool,

    /// Check files by MD5 and SHA512 hashes
    #[structopt(short = "h", long)]
    pub hash: bool,

    /// Check files by MD5 hash
    #[structopt(long = "md5")]
    pub hash_md5: bool,

    /// Check files by SHA512 hash
    #[structopt(long = "sha512")]
    pub hash_sha512: bool,

    /// Check files by content
    #[structopt(short = "t", long)]
    pub content: bool,

    /// Check files by datetime of creation
    #[structopt(short = "c", long)]
    pub date_created: bool,

    /// Check files by datetime of modification
    #[structopt(short = "m", long)]
    pub date_modified: bool,

    /// Minimum size of files to be checked
    #[structopt(long = "min-size", default_value = "0")]
    pub min_size: u64,

    /// Maximum size of files to be checked
    #[structopt(long = "max-size", default_value = "0")]
    pub max_size: u64,

    /// Minimum create date of files to be checked
    #[structopt(long = "min-create-date", default_value = "")]
    pub min_createdate: String,

    /// Maximum create date of files to be checked
    #[structopt(long = "max-create-date", default_value = "")]
    pub max_createdate: String,

    /// Minimum modify date of files to be checked
    #[structopt(long = "min-mod-date", default_value = "")]
    pub min_moddate: String,

    /// Maximum modify of files to be checked
    #[structopt(long = "max-mod-date", default_value = "")]
    pub max_moddate: String,

    /// File names filter
    #[structopt(long, default_value = "")]
    pub name_filter: String,

    /// First n files with maximum doubles to show
    #[structopt(short = "F", long = "first-n", default_value = "0")]
    pub first_n: usize,

    /// Sort results by name
    #[structopt(long = "sort-name")]
    pub sort_res_name_asc: bool,    

    /// Sort results by name in reverse order
    #[structopt(long = "sort-name-desc")]
    pub sort_res_name_desc: bool,  

    /// Sort results by size
    #[structopt(long = "sort-size")]
    pub sort_res_size_asc: bool,  

    /// Sort results by size in reverse order
    #[structopt(long = "sort-size-desc")]
    pub sort_res_size_desc: bool,  

    /// Sort results by create date
    #[structopt(long = "sort-create")]
    pub sort_res_cdate_asc: bool,  

    /// Sort results by create date in reverse order
    #[structopt(long = "sort-create-desc")]
    pub sort_res_cdate_desc: bool,  

    /// Sort results by name
    #[structopt(long = "sort-mod")]
    pub sort_res_mdate_asc: bool,  

    /// Sort results by name
    #[structopt(long = "sort-mod-desc")]
    pub sort_res_mdate_desc: bool,                              

    /// Log file
    #[structopt(short, long, default_value = "")]
    pub log: String,

    /// start path, . if not present
    #[structopt(parse(from_os_str))]
    path_start: Option<PathBuf>,

    /// output path, stdout if not present
    #[structopt(parse(from_os_str))]
    out: Option<PathBuf>,
}

pub fn init() -> Result<ConfigFile, confy::ConfyError> {
    let options = Options::from_args();
    //println!("{:#?}", options);

    let configfile: PathBuf = match options.configfile.to_str() {
        Some(x) => match x.is_empty() {
            true => PathBuf::from(DEFAULT_CFG),
            false => options.configfile,
        },
        None => PathBuf::from(DEFAULT_CFG),
    };

    let mut cfg: ConfigFile = load_path(configfile)?;

    cfg.silent_mode = options.silent_mode || cfg.silent_mode;

    cfg.global_verbose = if cfg.silent_mode {
        0
    } else {
        std::cmp::max(cfg.global_verbose, options.verbose)
    };

    cfg.hide_config = options.hide_config || cfg.hide_config;

    cfg.debug_config = (options.debug_config || cfg.debug_config) && !cfg.hide_config;

    cfg.debug = options.debug || cfg.debug;

    cfg.show_options_only = options.show_options_only || cfg.show_options_only;

    cfg.delete = (options.delete || cfg.delete) && !cfg.debug;
    cfg.force_delete = (options.force_delete || cfg.force_delete) && cfg.delete;

    cfg.name = options.name || cfg.name;
    cfg.size = options.size || cfg.size;
    cfg.date_created = options.date_created || cfg.date_created;
    cfg.date_modified = options.date_modified || cfg.date_modified;

    cfg.hash_md5 = match options.hash {
        true => true,
        false => options.hash_md5 || cfg.hash_md5,
    };
    cfg.hash_sha512 = match options.hash {
        true => true,
        false => options.hash_sha512 || cfg.hash_sha512,
    };
    cfg.content = options.content || cfg.content;

    cfg.min_size = match options.min_size {
        0 => cfg.min_size,
        s => s,
    };

    cfg.max_size = match options.max_size {
        0 => cfg.max_size,
        s => s,
    };

    cfg.min_createdate = if options.min_createdate == "" {
        cfg.min_createdate
    } else {
        options.min_createdate
    };

    cfg.max_createdate = if options.max_createdate == "" {
        cfg.max_createdate
    } else {
        options.max_createdate
    };

    cfg.min_moddate = if options.min_moddate == "" {
        cfg.min_moddate
    } else {
        options.min_moddate
    };

    cfg.max_moddate = if options.max_moddate == "" {
        cfg.max_moddate
    } else {
        options.max_moddate
    };

    cfg.first_n = match options.first_n {
        0 => cfg.first_n,
        s => s,
    };

    cfg.sort_res_name_asc = options.sort_res_name_asc || cfg.sort_res_name_asc;
    cfg.sort_res_name_desc = options.sort_res_name_desc || cfg.sort_res_name_desc;
    cfg.sort_res_size_asc = options.sort_res_size_asc || cfg.sort_res_size_asc;
    cfg.sort_res_size_desc = options.sort_res_size_desc || cfg.sort_res_size_desc;
    cfg.sort_res_cdate_asc = options.sort_res_cdate_asc || cfg.sort_res_cdate_asc;
    cfg.sort_res_cdate_desc = options.sort_res_cdate_desc || cfg.sort_res_cdate_desc;
    cfg.sort_res_mdate_asc = options.sort_res_mdate_asc || cfg.sort_res_mdate_asc;
    cfg.sort_res_mdate_desc = options.sort_res_mdate_desc || cfg.sort_res_mdate_desc;

    if cfg.sort_res_name_asc && cfg.sort_res_name_desc {
        panic!("Can't sort results by name in straight and reversed order simultaneously!");
    }

    if cfg.sort_res_size_asc && cfg.sort_res_size_desc {
        panic!("Can't sort results by size in straight and reversed order simultaneously!");
    }

    if cfg.sort_res_cdate_asc && cfg.sort_res_cdate_desc {
        panic!("Can't sort results by create date in straight and reversed order simultaneously!");
    }

    if cfg.sort_res_mdate_asc && cfg.sort_res_mdate_desc {
        panic!("Can't sort results by modification date in straight and reversed order simultaneously!");
    }

    cfg.name_filter = match options.name_filter.is_empty() {
        true => cfg.name_filter,
        false => options.name_filter,
    };

    cfg.path_start = match options.path_start {
        None => cfg.path_start,
        Some(x) => Some(x),
    };

    cfg.out_filename = match options.out {
        None => cfg.out_filename,
        Some(x) => {
            let now = Utc::now();
            let date_str = format!("{}{:02}{:02}", now.year(), now.month(), now.day());
            Some(PathBuf::from(
                x.into_os_string()
                    .into_string()
                    .unwrap()
                    .replace(DATE_TEMPLATE, &date_str),
            ))
        }
    };

    cfg.log_filename = match options.log.is_empty() {
        true => cfg.log_filename,
        false => options.log,
    };

    if !cfg.log_filename.is_empty() {
        let now = Utc::now();
        let date_str = format!("{}{:02}{:02}", now.year(), now.month(), now.day());
        cfg.log_filename = cfg.log_filename.replace(DATE_TEMPLATE, &date_str);
    }

    Ok(cfg)
}

pub fn init_log(log_filename: &String) -> Result<(), String> {
    let log = LogConfigBuilder::builder()
        .path(log_filename)
        .size(1 * 100)
        .roll_count(10)
        .level("info")
        .output_file()
        //.output_console()
        .build();

    simple_log::new(log)
}

pub fn convert_string_to_system_time(s: &String, err_msg: &str) -> SystemTime {
    match s.parse::<Timestamp>() {
        Ok(ts) => *ts,
        Err(e) => {
            panic!("{} {} - {}", err_msg, s, e);
        }
    }
}