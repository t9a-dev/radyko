use std::cmp::Reverse;

use bytes::Bytes;

pub struct AudioSegments {
    segments: Vec<AudioSegment>,
}

impl AudioSegments {
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    pub fn push_segment(&mut self, segment: AudioSegment) {
        self.segments.push(segment);
    }

    pub fn flush<W: std::io::Write>(mut self, writer: &mut W) -> anyhow::Result<()> {
        self.segments.sort_by_key(|a| Reverse(a.sequence));
        while let Some(AudioSegment {
            sequence: _,
            audio_bytes: bytes,
        }) = self.segments.pop()
        {
            writer.write_all(&bytes)?;
        }

        Ok(())
    }
}

impl Default for AudioSegments {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct AudioSegment {
    sequence: usize,
    audio_bytes: Bytes,
}

impl AudioSegment {
    pub fn new(sequence: usize, audio_bytes: Bytes) -> Self {
        Self {
            sequence,
            audio_bytes,
        }
    }
}

#[cfg(test)]
mod audio_segments_test {
    use std::io::Cursor;

    use bytes::Bytes;

    use crate::application::audio_segments::{AudioSegment, AudioSegments};

    #[test]
    fn flush_entries_in_ascending_sequence_order() -> anyhow::Result<()> {
        let mut audio_segments = AudioSegments::new();
        audio_segments.push_segment(AudioSegment::new(2, Bytes::from("b")));
        audio_segments.push_segment(AudioSegment::new(1, Bytes::from("a")));
        audio_segments.push_segment(AudioSegment::new(3, Bytes::from("c")));

        let mut writer = Cursor::new(Vec::new());
        audio_segments.flush(&mut writer)?;

        assert_eq!(writer.get_ref(), b"abc");
        Ok(())
    }
}
