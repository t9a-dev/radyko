use serde_derive::Deserialize;
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
