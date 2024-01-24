use std::time::SystemTime;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CheckOptions {
    pub name: Option<String>,
    pub size: Option<u64>,
    pub created: Option<SystemTime>,
    pub modified: Option<SystemTime>,
    pub md5: Option<String>,
    pub sha512: Option<String>,
}
