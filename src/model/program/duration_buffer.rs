use crate::app::{config::RecordingDurationBufferConfig, types::Seconds};

#[derive(Debug, Clone, Copy)]
pub struct RecordingDurationBuffer {
    start: Seconds,
    end: Seconds,
}

impl RecordingDurationBuffer {
    pub fn new(start: Seconds, end: Seconds) -> Self {
        Self { start, end }
    }

    pub fn from_config(config: Option<RecordingDurationBufferConfig>) -> Self {
        match config {
            Some(config) => Self {
                start: Seconds::new(config.start),
                end: Seconds::new(config.end),
            },
            None => Self::default(),
        }
    }

    pub fn start(self) -> Seconds {
        self.start
    }

    pub fn end(self) -> Seconds {
        self.end
    }
}

impl Default for RecordingDurationBuffer {
    fn default() -> Self {
        Self {
            start: Seconds::new(0),
            end: Seconds::new(0),
        }
    }
}
