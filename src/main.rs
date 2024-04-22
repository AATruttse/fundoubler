#[macro_use]
extern crate simple_log;

use std::fs::File;
use std::io::Read;
use std::time::SystemTime;

use dialoguer::Confirm;
use itertools::Itertools;
use multimap::MultiMap;
use regex::Regex;
use sha2::{Digest, Sha512};
use walkdir::WalkDir;

use init::{convert_string_to_system_time, init_log, ConfigFile};
use crate::check::{compare, CheckOptions};

mod check;
mod init;

fn analyze(cfg: &ConfigFile) -> MultiMap<CheckOptions, CheckOptions> {
    let mut files: MultiMap<check::CheckOptions, CheckOptions> = MultiMap::new();

    let min_create_date = if cfg.min_createdate == "" {
        None
    } else {
        Some(convert_string_to_system_time(
            &cfg.min_createdate,
            "Can't parse min_createdate",
        ))
    };

    let max_create_date = if cfg.max_createdate == "" {
        None
    } else {
        Some(convert_string_to_system_time(
            &cfg.max_createdate,
            "Can't parse max_createdate",
        ))
    };

    let min_mod_date = if cfg.min_moddate == "" {
        None
    } else {
        Some(convert_string_to_system_time(
            &cfg.min_moddate,
            "Can't parse min_moddate",
        ))
    };

    let max_mod_date = if cfg.max_moddate == "" {
        None
    } else {
        Some(convert_string_to_system_time(
            &cfg.max_moddate,
            "Can't parse max_moddate",
        ))
    };

    let re = if cfg.name_filter == "" {
        None
    } else {
        match Regex::new(cfg.name_filter.as_str()) {
            Ok(r) => Some(r),
            Err(e) => {
                panic!("Can't parse filename filter from regexp {} - {}", cfg.name_filter, e);
            }
        }
    };

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
        let mut file_date_c: Option<SystemTime> = None;
        let mut file_date_m: Option<SystemTime> = None;
        let mut file_md5: Option<String> = None;
        let mut file_sha512: Option<String> = None;

        let file_name = String::from(entry.file_name().to_string_lossy());
        let file_path = String::from(entry.path().to_string_lossy());

        if cfg.global_verbose > 0 {
            info!("{}", &file_path);
            println!("{}", &file_path);
        }

        let file_metadata = match entry.metadata() {
            Ok(m) => m,
            Err(e) => {
                warn!("Can't get metadata for file {}. {}", &file_path, e);
                continue;
            }
        };

        let mut file_opt : CheckOptions = CheckOptions::new();
        file_opt.name = Some(file_path.clone());
 
        file_opt.size = Some(file_metadata.len());
        let file_size = if cfg.size {
            file_opt.size
        } else {
            None
        };

        file_opt.created = file_metadata.created().ok();
        if cfg.date_created {
            file_date_c = file_opt.created;
        };

        file_opt.modified = file_metadata.modified().ok();
        if cfg.date_modified {
            file_date_m = file_opt.modified;
        };        

        if (cfg.min_size > 0 && file_metadata.len() < cfg.min_size)
            || (cfg.max_size > 0 && file_metadata.len() > cfg.max_size)
            || (min_create_date.is_some()
                && file_date_c.is_some()
                && file_date_c.unwrap() > min_create_date.unwrap())
            || (max_create_date.is_some()
                && file_date_c.is_some()
                && file_date_c.unwrap() < max_create_date.unwrap())
            || (min_mod_date.is_some()
                && file_date_m.is_some()
                && file_date_m.unwrap() > min_mod_date.unwrap())
            || (max_mod_date.is_some()
                && file_date_m.is_some()
                && file_date_m.unwrap() > max_mod_date.unwrap())
            || (re.is_some()
                && !re.as_ref().unwrap().is_match(&file_name.as_str()) 
            )
        {
            continue;
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
            name: Some(file_name),
            size: file_size,
            created: file_date_c,
            modified: file_date_m,
            md5: file_md5,
            sha512: file_sha512,
        };


        files.insert(file_key, file_opt);
    }

    if cfg.debug {
        println!("{:?}", files);
    }

    let vals = files
        .iter_all()
        .filter(|(_, v)| v.len() > 1)
        .map(|(k, v)| (k.clone(), v.clone()
            .into_iter()
            .sorted_by(|v0, v1| compare(cfg, v0, v1)).collect()))
        .sorted_by(|(k0, _), (k1, _)| compare(cfg, k0, k1));

    if cfg.first_n > 0 {
        return MultiMap::from_iter(vals.take(cfg.first_n).collect::<Vec<(CheckOptions, Vec<CheckOptions>)>>());
    }
    
    MultiMap::from_iter(vals.collect::<Vec<(CheckOptions, Vec<CheckOptions>)>>())
}

fn show_results(results: &MultiMap<CheckOptions, CheckOptions>) {
    for (opt, pathes) in results.iter_all() {
        println!("{}", opt);
        for path in pathes.iter() {
            println!("    {}", path.name.clone().unwrap_or_default());
        }
    }
}

fn delete_results(cfg: &ConfigFile, results: &MultiMap<CheckOptions, CheckOptions>) {
    if !cfg.delete {
        return;
    }

    for (template, files) in results.iter_all() {
        info!("{}", template);
        if !cfg.silent_mode || !cfg.force_delete {
            println!("{}", template);
        }

        let mut num_del: usize = 0;
        let mut idx_file: usize = 0;
        let max_del = files.len() - 1;
        
        for file in files.iter() {
            idx_file += 1;

            println!("    {}...   ", file);
            
            if cfg.force_delete {
                if !cfg.silent_mode {
                    print!("    {}...   ", file);
                }
                if idx_file == 1 {
                    if !cfg.silent_mode {
                        println!("keep!");
                    }
                    info!("    {} - keep!", file);
                    continue;
                }
            }

            if !cfg.force_delete {
                let prompt = format! {"    {} delete (y/n)?", file};

                if num_del == max_del
                    || !Confirm::new()
                        .with_prompt(prompt)
                        .default(true)
                        .show_default(true)
                        .interact()
                        .unwrap()
                {
                    info!("    {} - keep!", file);
                    continue;
                }
            }

            if cfg.force_delete {
                if !cfg.silent_mode {
                    println!("delete!");
                }
            }
            info!("    {} - delete!", file);

            let path_to_del = match &file.name {
                Some(s) => s,
                None => {
                    println!("Can't get path from {}", file);
                    warn!("Can't get path from {}", file);
                    continue;
                }
            };

            if !cfg.debug {
                match std::fs::remove_file(path_to_del) {
                    Ok(_) => {
                        num_del += 1;
                    }
                    Err(e) => {
                        println!("Can't delete {} - {}", path_to_del, e);
                        warn!("Can't delete {} - {}", path_to_del, e);
                    }
                }
            }
        }
    }
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

    if !cfg.silent_mode {
        show_results(&file_doubles);
    }

    if cfg.delete {
        delete_results(&cfg, &file_doubles);
    }

    Ok(())
}
