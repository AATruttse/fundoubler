extern crate chrono;

use std::cmp::Ordering;
use std::fmt;
use std::time::SystemTime;

use chrono::offset::Utc;
use chrono::DateTime;

use crate::init::ConfigFile;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CheckOptions {
    pub name: Option<String>,
    pub size: Option<u64>,
    pub created: Option<SystemTime>,
    pub modified: Option<SystemTime>,
    pub md5: Option<String>,
    pub sha512: Option<String>,
}

impl CheckOptions {
    pub fn new() -> Self {
        Self {
            name: None,
            size: None,
            created: None,
            modified: None,
            md5: None,
            sha512: None,
        }
    }
}

pub fn compare(cfg: &ConfigFile, opt0: &CheckOptions, opt1: &CheckOptions) -> Ordering {
    if cfg.sort_res_name_asc || cfg.sort_res_name_desc {
        let cmp_name = if cfg.sort_res_name_asc {
            opt0.name.partial_cmp(&opt1.name).unwrap()
        } else {
            opt1.name.partial_cmp(&opt0.name).unwrap()
        };
        if cmp_name != Ordering::Equal {
            return cmp_name;
        }
    }

    if cfg.sort_res_size_asc || cfg.sort_res_size_desc {
        let cmp_size = if cfg.sort_res_size_asc {
            opt0.size.partial_cmp(&opt1.size).unwrap()
        } else {
            opt1.size.partial_cmp(&opt0.size).unwrap()
        };
        if cmp_size != Ordering::Equal {
            return cmp_size;
        }
    }

    if cfg.sort_res_cdate_asc || cfg.sort_res_cdate_desc {
        let cmp_cdate = if cfg.sort_res_cdate_asc {
            opt0.created.partial_cmp(&opt1.created).unwrap()
        } else {
            opt1.created.partial_cmp(&opt0.created).unwrap()
        };
        if cmp_cdate != Ordering::Equal {
            return cmp_cdate;
        }
    }

    if cfg.sort_res_mdate_asc || cfg.sort_res_mdate_desc {
        let cmp_mdate = if cfg.sort_res_mdate_asc {
            opt0.modified.partial_cmp(&opt1.modified).unwrap()
        } else {
            opt1.modified.partial_cmp(&opt0.modified).unwrap()
        };
        if cmp_mdate != Ordering::Equal {
            return cmp_mdate;
        }
    }

    return Ordering::Equal;
}

impl fmt::Display for CheckOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;

        if self.name.is_some() {
            write!(f, "{}", &self.name.as_ref().unwrap())?;
            first = false;
        }

        if self.size.is_some() {
            if !first {
                write!(f, " - ")?;
            }
            write!(f, "{}", &self.size.as_ref().unwrap())?;
            first = false;
        }

        if self.created.is_some() {
            if !first {
                write!(f, ", ")?;
            }

            let datetime: DateTime<Utc> = self.created.unwrap().into();
            write!(f, "created: {}", datetime.format("%Y-%m-%d][%H:%M:%S"))?;
            first = false;
        }

        if self.modified.is_some() {
            if !first {
                write!(f, ", ")?;
            }

            let datetime: DateTime<Utc> = self.modified.unwrap().into();
            write!(f, "modified: {}", datetime.format("%Y-%m-%d][%H:%M:%S"))?;
            first = false;
        }

        if self.md5.is_some() {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "MD5: {}", &self.md5.as_ref().unwrap())?;
            first = false;
        }

        if self.sha512.is_some() {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "SHA512: {}", &self.md5.as_ref().unwrap())?;
        }

        Ok(())
    }
}
