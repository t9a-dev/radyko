use crate::app::{config::RecordingDurationBufferConfig, types::Seconds};

#[derive(Debug, Clone, Copy)]
pub struct RecordingDurationBuffer {
    pub(crate) start: Seconds,
    pub(crate) end: Seconds,
}

impl RecordingDurationBuffer {
    pub fn new(start: Seconds, end: Seconds) -> Self {
        Self { start, end }
    }
    pub fn from_config(config: Option<RecordingDurationBufferConfig>) -> Self {
        match config {
            Some(config) => Self {
                start: Seconds(config.start),
                end: Seconds(config.end),
            },
            None => Self::default(),
        }
    }
}

impl Default for RecordingDurationBuffer {
    fn default() -> Self {
        Self {
            start: Seconds(0),
            end: Seconds(0),
        }
    }
}
