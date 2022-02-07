use std::fmt::{Display, Formatter};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct TrackData {
    pub path:PathBuf,
    pub artist:Option<String>,
    pub album:Option<String>,
    pub title:Option<String>,
    pub number:Option<String>,
    pub total:Option<String>,
}

impl Display for TrackData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut num = 0;
        let mut tot = 0;
        let str = match &self.title {
            None => "unknown",
            Some(data) => data,
        };
        if let Some(st) = &self.total {
            if let Ok(n) = st.parse::<i32>() {
                tot = n;
            }
        }
        if let Some(st) = &self.number {
            if st.contains('/') {
                if let Some(n) = st.find('/') {
                    for (i,part) in st.split('/').enumerate() {
                        if i == 0 {
                            if let Ok(n) = part.parse::<i32>() {
                                num = n;
                            }
                        }
                        if i == 1 {
                            if let Ok(n) = part.parse::<i32>() {
                                tot = n;
                            }
                        }
                    }
                }
            } else {
                if let Ok(n) = st.parse::<i32>() {
                    num = n;
                }
            }
        }
        f.write_str(&format!("{:02}/{:02} {}", num, tot, str))
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
