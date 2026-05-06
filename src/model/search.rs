use serde_derive::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use strum_macros::{AsRefStr, Display};

#[derive(Debug, Clone, Copy, Display, AsRefStr, Serialize, Deserialize)]
pub enum Filter {
    #[strum(to_string = "future")]
    Live,
    #[strum(to_string = "")]
    All,
    #[strum(to_string = "past")]
    Timefree,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchCondition {
    pub key: Vec<String>,
    pub filter: Option<Filter>,
    pub start_day: Option<String>,
    pub end_day: Option<String>,
    pub row_limit: Option<i32>,
    pub area_id: Option<Vec<String>>,
    pub station_id: Option<Vec<String>>,
    pub cur_area_id: Option<String>,
}
