use std::fmt::{Display, Formatter};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct TrackData {
    pub path:PathBuf,
    pub artist:Option<String>,
    pub album:Option<String>,
    pub title:Option<String>,
}

impl Display for TrackData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match &self.title {
            None => "unknown",
            Some(data) => data,
        };
        f.write_str(str)
    }
}


pub fn get_or<'a>(item:&'a Option<String>, backup:&'a str) -> &'a str {
    let val:&str = if let Some(item) = &item {
        item
    } else {
        backup
    };
    val
}
