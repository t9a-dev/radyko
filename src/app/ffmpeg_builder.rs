use std::fmt::Debug;
use std::process::Stdio;
use std::sync::Arc;

use anyhow::anyhow;
use tokio::io::{AsyncBufReadExt, BufReader};

use strum_macros::AsRefStr;
use strum_macros::EnumString;
use tokio::process::Child;
use tokio::process::Command;
use tokio::sync::RwLock;

use tracing::{debug, error, info, trace, warn};

use crate::app::ffmpeg_builder::types::FfmpegOutput;
use crate::model::program::Seconds;

mod types {
    // FfmpegOutput("abc")のようなnew()を経由しない利用を防ぐためmodに分離

    #[derive(Debug, Clone)]
    pub struct FfmpegOutput(String);
    impl FfmpegOutput {
        pub fn new(path: String) -> Self {
            if path.ends_with(".aac") {
                return Self(path);
            }

            Self(format!("{path}.aac"))
        }

        pub fn path(self) -> String {
            self.0
        }
    }
}

#[derive(Debug, Clone, AsRefStr, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum LogLevel {
    Quiet,
    Panic,
    Fatal,
    Error,
    Warning,
    Info,
    Verbose,
    Debug,
    Trace,
}

#[derive(Debug, Clone)]
pub struct FfmpegBuilder {
    headers: Vec<String>,
    input: Option<String>,
    output: Option<FfmpegOutput>,
    codec: Option<String>,
    additional_args_before_input: Vec<String>,
    additional_args_after_input: Vec<String>,
    loglevel: LogLevel,
}

#[allow(dead_code)]
impl Default for FfmpegBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FfmpegBuilder {
    pub const HLS_PLAYLIST_FILE_NAME: &str = "playlist.m3u8";

    pub fn new() -> Self {
        Self {
            headers: Vec::new(),
            input: None,
            output: None,
            codec: None,
            additional_args_before_input: Vec::new(),
            additional_args_after_input: Vec::new(),
            loglevel: LogLevel::Error,
        }
    }

    pub fn header<S: Into<String>>(mut self, key: S, value: S) -> Self {
        self.headers.push("-headers".to_string());
        self.headers
            .push(format!("{}:{}\r\n", key.into(), value.into()));

        self
    }

    pub fn input<S: Into<String>>(mut self, input: S) -> Self {
        self.input = Some(input.into());
        self
    }

    pub fn output<S: Into<String>>(mut self, output: S) -> Self {
        self.output = Some(FfmpegOutput::new(output.into()));
        self
    }

    pub fn codec<S: Into<String>>(mut self, codec: S) -> Self {
        self.codec = Some(codec.into());
        self
    }

    pub fn additional_args<S: Into<String>>(mut self, arg: S) -> Self {
        self.additional_args_after_input.push(arg.into());
        self
    }

    pub fn for_hls_streaming<S: Into<String>>(mut self, segment_pattern: S) -> Self {
        let segment_pattern = segment_pattern.into();
        let mut args_before_input = vec![
            "-protocol_whitelist",
            "file,http,https,tcp,tls,crypto",
            "-allowed_extensions",
            "ALL",
            "-seekable",
            "0",
            "-http_seekable",
            "0",
        ]
        .into_iter()
        .map(|arg| arg.to_string());

        let mut args_after_input = vec![
            "-hls_time",
            "5",
            "-hls_list_size",
            "5",
            "-hls_segment_filename",
            &segment_pattern,
            "-f",
            "hls",
        ]
        .into_iter()
        .map(|arg| arg.to_string());

        self.additional_args_before_input
            .extend(&mut args_before_input);
        self.additional_args_after_input
            .extend(&mut args_after_input);

        self
    }

    pub fn for_hls_recording(mut self, recording_duration: Seconds) -> Self {
        let recording_duration_secs = recording_duration.0.to_string();
        let mut args_before_input = vec![
            "-protocol_whitelist",
            "file,http,https,tcp,tls,crypto",
            "-allowed_extensions",
            "ALL",
            "-seekable",
            "0",
            "-http_seekable",
            "0",
        ]
        .into_iter()
        .map(|arg| arg.to_string());

        let mut args_after_input = vec![
            "-reconnect",
            "3",
            "-reconnect_at_eof",
            "1",
            "-reconnect_streamed",
            "1",
            "-reconnect_delay_max",
            "5",
            "-timeout",
            "10000000",
            "-http_persistent",
            "1",
            "-acodec",
            "copy",
            "-vn",
            "-bsf:a",
            "aac_adtstoasc",
            "-y",
            "-t",
            &recording_duration_secs,
        ]
        .into_iter()
        .map(|arg| arg.to_string());

        self.additional_args_before_input
            .extend(&mut args_before_input);
        self.additional_args_after_input
            .extend(&mut args_after_input);

        self
    }

    pub fn loglevel(mut self, loglevel: LogLevel) -> Self {
        self.additional_args_after_input
            .push("-loglevel".to_string());
        self.additional_args_after_input
            .push(loglevel.as_ref().to_lowercase());
        self.loglevel = loglevel;

        self
    }

    pub async fn build(self) -> anyhow::Result<Arc<RwLock<Child>>> {
        let input = self.input.ok_or(anyhow!("Input is required."))?;
        let output = self.output.ok_or(anyhow!("Output is required"))?;

        let mut cmd = Command::new("ffmpeg");
        cmd.args(self.headers.iter().collect::<Vec<_>>());
        cmd.args(self.additional_args_before_input.iter().collect::<Vec<_>>());
        cmd.arg("-i").arg(input);

        match self.codec {
            Some(codec) => cmd.arg("-c:a").arg(codec),
            None => cmd.arg("-c:a").arg("copy"),
        };

        cmd.args(self.additional_args_after_input.iter().collect::<Vec<_>>());
        cmd.arg(output.path());

        let child = Arc::new(RwLock::new(
            cmd.stdout(Stdio::null())
                .stderr(Stdio::piped())
                .kill_on_drop(false)
                .spawn()?,
        ));

        trace!("cmd info: {:#?}", cmd);

        Ok(child)
    }

    #[allow(dead_code)]
    async fn handle_stdio(child: Arc<RwLock<Child>>, loglevel: LogLevel) {
        let mut child_mut = child.write().await;
        // FFmpegではログがstderrに出力される
        let stdio_task = child_mut.stderr.take().map(|stderr| {
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Some(line) = lines.next_line().await.unwrap() {
                    match loglevel {
                        LogLevel::Quiet => (),
                        LogLevel::Panic => error!("panic: {}", line),
                        LogLevel::Fatal => error!("fatal: {}", line),
                        LogLevel::Error => error!("error: {}", line),
                        LogLevel::Warning => warn!("warning: {}", line),
                        LogLevel::Info => info!("info: {}", line),
                        LogLevel::Verbose => debug!("verbose: {}", line),
                        LogLevel::Debug => debug!("debug: {}", line),
                        LogLevel::Trace => trace!("trace: {}", line),
                    }
                }
            })
        });

        if let Some(t) = stdio_task {
            t.await.unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use tempfile::NamedTempFile;

    use crate::{
        app::ffmpeg_builder::FfmpegBuilder, constants::test_constants::TEST_STATION_ID,
        model::program::Seconds, radiko::RadikoClient, test_helper::radiko_client,
    };

    async fn build_hls_recording_cmd(
        radiko_client: &RadikoClient,
    ) -> anyhow::Result<Arc<RwLock<tokio::process::Child>>> {
        let auth_token = radiko_client.auth_token().await;
        let temp_file = NamedTempFile::new()?;
        let temp_file_path = temp_file.path().to_str().unwrap();
        let stream_url = radiko_client.stream_url(TEST_STATION_ID).await;
        let ffmpeg_builder = FfmpegBuilder::new();

        Ok(ffmpeg_builder
            .header("X-Radiko-Authtoken", &auth_token)
            .input(stream_url)
            .for_hls_recording(Seconds(1))
            .output(temp_file_path)
            .build()
            .await?)
    }

    async fn build_hls_stream_cmd(
        radiko_client: &RadikoClient,
    ) -> anyhow::Result<Arc<RwLock<tokio::process::Child>>> {
        let auth_token = radiko_client.auth_token().await;
        let stream_url = radiko_client.stream_url(TEST_STATION_ID).await;
        let temp_path = tempfile::Builder::new()
            .prefix(&format!("hls_stream_{}_", TEST_STATION_ID))
            .tempdir()
            .map_err(|e| anyhow!("Failed to create temp directory: {}", &e))?;
        let output_playlist = temp_path.path().join(FfmpegBuilder::HLS_PLAYLIST_FILE_NAME);
        let segment_pattern = temp_path.path().join("segment_%10d.aac");
        let ffmpeg_builder = FfmpegBuilder::new();

        Ok(ffmpeg_builder
            .header("X-Radiko-Authtoken", &auth_token)
            .input(stream_url)
            .for_hls_streaming(segment_pattern.to_str().unwrap())
            .additional_args("-t")
            .additional_args("1")
            .output(output_playlist.as_path().to_string_lossy())
            .build()
            .await?)
    }

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn ffmpeg_builder_for_hls_recording_smoke() -> anyhow::Result<()> {
        let cmd = build_hls_recording_cmd(radiko_client().await).await?;
        let mut child = cmd.write().await;

        assert!(child.wait().await?.success());
        Ok(())
    }

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn ffmpeg_builder_for_hls_recording_area_free_smoke() -> anyhow::Result<()> {
        let cmd = build_hls_recording_cmd(radiko_client().await).await?;
        let mut child = cmd.write().await;

        assert!(child.wait().await?.success());
        Ok(())
    }

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn ffmpeg_builder_for_hls_streaming_smoke() -> anyhow::Result<()> {
        let cmd = build_hls_stream_cmd(radiko_client().await).await?;
        let mut child = cmd.write().await;

        assert!(child.wait().await?.success());
        Ok(())
    }

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn ffmpeg_builder_for_hls_streaming_area_free_smoke() -> anyhow::Result<()> {
        let cmd = build_hls_stream_cmd(radiko_client().await).await?;
        let mut child = cmd.write().await;

        assert!(child.wait().await?.success());
        Ok(())
    }
}
