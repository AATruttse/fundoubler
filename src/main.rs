#[macro_use]
extern crate simple_log;

use std::fs::File;
use std::io::Read;
use std::time::SystemTime;

use init::{init_log, ConfigFile};
use itertools::Itertools;
use multimap::MultiMap;
use sha2::{Digest, Sha512};
use walkdir::WalkDir;

use crate::check::CheckOptions;

mod check;
mod init;

fn analyze(cfg: &ConfigFile) -> MultiMap<CheckOptions, String> {
    let mut files: MultiMap<check::CheckOptions, String> = MultiMap::new();

    for entry in WalkDir::new(
        cfg.path_start
            .clone()
            .expect("")
            .into_os_string()
            .into_string()
            .unwrap(),
    )
    .into_iter()
    .filter_map(Result::ok)
    .filter(|e| !e.file_type().is_dir())
    {
        let mut file_name: Option<String> = None;
        let mut file_size: Option<u64> = None;
        let mut file_date_c: Option<SystemTime> = None;
        let mut file_date_m: Option<SystemTime> = None;
        let mut file_md5: Option<String> = None;
        let mut file_sha512: Option<String> = None;

        let file_path = String::from(entry.path().to_string_lossy());

        if cfg.global_verbose > 0 {
            info!("{}", &file_path);
            println!("{}", &file_path);
        }

        if cfg.name || cfg.global_verbose > 0 {
            file_name = Some(String::from(entry.file_name().to_string_lossy()));
        }

        if cfg.size || cfg.date_created || cfg.date_modified {
            let file_metadata = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    warn!("Can't get metadata for file {}. {}", &file_path, e);
                    continue;
                }
            };
            file_size = Some(file_metadata.len());
            file_date_c = file_metadata.created().ok();
            file_date_m = file_metadata.modified().ok();
        }

        if cfg.hash_md5 || cfg.hash_sha512 {
            let mut file = match File::open(file_path.clone()) {
                Ok(md5) => md5,
                Err(e) => {
                    warn!("Can't open file {}. {}", file_path, e);
                    continue;
                }
            };
            let mut contents = Vec::<u8>::new();
            if file.read_to_end(&mut contents).is_err() {
                warn!("Can't read file {}", file_path);
                continue;
            }

            if cfg.hash_md5 {
                let md5_digest = md5::compute(&contents.as_slice());
                file_md5 = Some(format!("{:x}", md5_digest));
            }

            if cfg.hash_sha512 {
                let mut hasher = Sha512::new();
                hasher.update(&contents.as_slice());
                let sha512_digest = hasher.finalize();
                file_sha512 = Some(format!("{:x}", sha512_digest));
            }
        }

        let file_key = check::CheckOptions {
            name: file_name,
            size: file_size,
            created: file_date_c,
            modified: file_date_m,
            md5: file_md5,
            sha512: file_sha512,
        };

        files.insert(file_key, file_path);
    }

    let vals = files
        .iter_all()
        .filter(|(_, v)| v.len() > 1)
        .map(|(k, v)| (k.clone(), v.clone()))
        .sorted()
        .collect::<Vec<(CheckOptions, Vec<String>)>>();
    MultiMap::from_iter(vals)
}

fn main() -> Result<(), String> {
    //let start = Instant::now();
    let cfg = init::init().expect("Config file error");

    if init_log(&cfg.log_filename).is_err() {
        panic!("Can't init log file {}", &cfg.log_filename);
    }

    if cfg.debug_config {
        println!("Config options:");
        println!("{:#?}", cfg);
    }

    if cfg.show_options_only {
        std::process::exit(0);
    }

    if !cfg.name
        && !cfg.size
        && !cfg.date_created
        && !cfg.date_modified
        && !cfg.hash_md5
        && !cfg.hash_sha512
    {
        error!("Need to turn on at least one file equality criteria");
        panic!("Need to turn on at least one file equality criteria")
    }

    let file_doubles = analyze(&cfg);
    println!("{:?}", file_doubles);
    Ok(())
}
