use std::time::Duration;

use crate::app::config::RecordingDurationBufferConfig;

#[derive(Debug, Clone, Copy)]
pub struct RecordingDurationBuffers {
    start_buffer: StartBuffer,
    end_buffer: EndBuffer,
}

impl RecordingDurationBuffers {
    pub fn new(start_buffer: StartBuffer, end_buffer: EndBuffer) -> Self {
        Self {
            start_buffer,
            end_buffer,
        }
    }

    pub fn from_config(config: Option<RecordingDurationBufferConfig>) -> Self {
        match config {
            Some(config) => Self {
                start_buffer: StartBuffer::new(Duration::from_secs(config.start)),
                end_buffer: EndBuffer::new(Duration::from_secs(config.end)),
            },
            None => Self::default(),
        }
    }

    pub fn start_buffer(self) -> StartBuffer {
        self.start_buffer
    }

    pub fn end_buffer(self) -> EndBuffer {
        self.end_buffer
    }
}

impl Default for RecordingDurationBuffers {
    fn default() -> Self {
        Self {
            start_buffer: StartBuffer::new(Duration::from_secs(0)),
            end_buffer: EndBuffer::new(Duration::from_secs(0)),
        }
    }
}
pub trait BufferSecs {
    fn new(duration: Duration) -> Self
    where
        Self: Sized;

    fn secs(&self) -> u64;

    fn duration(&self) -> Duration;
}

#[derive(Debug, Clone, Copy)]
pub struct StartBuffer(Duration);

impl BufferSecs for StartBuffer {
    fn new(duration: Duration) -> Self
    where
        Self: Sized,
    {
        Self(duration)
    }

    fn secs(&self) -> u64 {
        self.0.as_secs()
    }

    fn duration(&self) -> Duration {
        self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EndBuffer(Duration);

impl BufferSecs for EndBuffer {
    fn new(duration: Duration) -> Self
    where
        Self: Sized,
    {
        Self(duration)
    }

    fn secs(&self) -> u64 {
        self.0.as_secs()
    }

    fn duration(&self) -> Duration {
        self.0
    }
}
