use serde_derive::Deserialize;
use serde_with::skip_serializing_none;
use strum_macros::{AsRefStr, Display};

#[derive(Debug, Clone, Copy, Display, AsRefStr, Deserialize)]
pub enum Filter {
    #[strum(to_string = "future")]
    Live,
    #[strum(to_string = "")]
    All,
    #[strum(to_string = "past")]
    Timefree,
}

#[allow(dead_code)]
#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize)]
pub struct SearchCondition {
    key: Vec<String>,
    filter: Option<Filter>,
    start_day: Option<String>,
    end_day: Option<String>,
    row_limit: Option<i32>,
    area_id: Option<Vec<String>>,
    station_id: Option<Vec<String>>,
    cur_area_id: Option<String>,
}
