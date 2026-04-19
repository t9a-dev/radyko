/*
  MEMO:
  radikoのストリーミング配信はHLS(HTTP Live Streaming)で行われている
  https://www.rfc-editor.org/rfc/rfc8216
  #EXT-X-MEDIA-SEQUENCE:24484340 <- ここがインデックスのベース
  medialistにはセグメントのURLが3つ並んでいる。
  先頭をインデックス0として、ベースインデックスとプラスして管理する
  last_process_sequence として #EXT-X-MEDIA-SEQUENCE + indexを保持しておく
  last_process_sequence より大きい場合に処理対象とすることで、一度処理したセグメントをスキップできる

  HLS(HTTP Live Streaming) Packed Audioについて
  radikoのHLSでは5秒の.aac音声ファイルがセグメントとして配信されている
  この音声ファイルを結合することで録音が可能になる
  しかし、そのままセグメントファイルを結合して再生するとノイズが発生する
  セグメントが純粋な音声データだけでなく、メタデータを含むため
  メタデータを取り除いて、音声ファイルのみを結合する必要がある

  $ xxd -g 1 20260410_075630_xv6d3.aac | head
  - 16進数で表示されている
    - 1Byte = 8bit で　4bit 4bit の並び
      - 0x49のとき　0100 1001
        - 0xが16進数であることを表している
  - 0x49 0x44 0x33 : "ID3" ASCII TableのHEX(16進数)列が対応している
    - https://www.ascii-code.com/
  - 仕様書からID3 tag 全体のサイズがわかるので、この部分のデータをスキップすると音声データが始まる
    - https://id3.org/id3v2.4.0-structure

  // 00000040:は音声データなので$で置換している
  00000000: 49 44 33 04 00 00 00 00 00 3f 50 52 49 56 00 00  ID3......?PRIV..
  00000010: 00 35 00 00 63 6f 6d 2e 61 70 70 6c 65 2e 73 74  .5..com.apple.st
  00000020: 72 65 61 6d 69 6e 67 2e 74 72 61 6e 73 70 6f 72  reaming.transpor
  00000030: 74 53 74 72 65 61 6d 54 69 6d 65 73 74 61 6d 70  tStreamTimestamp
  00000040: 00 00 00 00 01 b7 0f cd 00 ff f9 $$ $$ $$ $$ $$  ...........X....
  00000050: $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$  !..$...$..$.....
  00000060: $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$  ..$.....;.$"....
  00000070: $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$  ..\..~^^.....$..
  00000080: $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$  $}$..$$$$$..]..$
  00000090: $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$ $$  .>$.$....!....[.

*/

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, bail};
use bytes::{Buf, Bytes};
use hls_m3u8::MediaPlaylist;
use tokio::{
    io::AsyncWriteExt,
    sync::mpsc::{self, Receiver, Sender},
    time::Instant,
};
use tracing::{Instrument, error, info, info_span, trace, warn};

#[derive(Debug, Clone)]
pub struct StreamHandler {
    inner: Arc<StreamHandlerRef>,
}

#[derive(Debug)]
struct StreamHandlerRef {
    client: reqwest::Client,
    media_list_url: String,
}

impl StreamHandler {
    pub fn new(client: reqwest::Client, media_list_url: String) -> Self {
        Self {
            inner: Arc::new(StreamHandlerRef {
                client,
                media_list_url,
            }),
        }
    }

    pub async fn start_recording(
        &self,
        output_dir: PathBuf,
        file_name: &str,
        recording_duration: Duration,
    ) -> anyhow::Result<()> {
        let end_recording = Instant::now() + recording_duration;
        tokio::fs::create_dir_all(output_dir.clone()).await?;
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .write(true)
            .open(Path::new(&output_dir).join(file_name))
            .await?;

        let span = info_span!("handle_stream_start_recording", program = file_name);
        let mut audio_segments_receiver = self.start_read().instrument(span).await?;
        while let Some(audio_segment) = audio_segments_receiver.recv().await {
            if end_recording <= Instant::now() {
                info!("end recording: {}", file_name);
                drop(audio_segment);
                drop(audio_segments_receiver);
                return Ok(());
            }

            let audio_segment = audio_segment?;
            file.write_all(&audio_segment).await?;
            file.flush().await?;
            trace!("recive segment len: {}", audio_segment.len());
            drop(audio_segment);
        }

        Ok(())
    }

    async fn start_read(&self) -> anyhow::Result<Receiver<anyhow::Result<Bytes>>> {
        trace!("start read");
        // radikoのHLSで返されるセグメント数が常に3なので適当に2倍を見ておいてbufferを6にした
        let (tx, rx) = mpsc::channel::<anyhow::Result<Bytes>>(6);

        let this = self.clone();
        tokio::spawn(
            async move {
                // エラーをチャネル経由で伝搬する
                // handle_hls_stream自体は無限ループなので正常系で値を返さないのでエラーのみ処理
                if let Err(e) = this.handle_hls_stream(tx.clone()).await {
                    error!("handle hls stream error: {:#?}", e);
                    let _ = tx.send(Err(e)).await;
                }
            }
            .in_current_span(),
        );

        Ok(rx)
    }

    async fn handle_hls_stream(self, tx: Sender<anyhow::Result<Bytes>>) -> anyhow::Result<()> {
        trace!("start handle hls stream");
        let mut last_processed_sequence = 0;

        loop {
            // media_list_urlは再読込すると新しいセグメントの配信URLが返ってくる
            let media_playlist_response = self
                .inner
                .client
                .get(&self.inner.media_list_url)
                .send()
                .await
                .context("faild get playlist")?;
            let media_playlist_content = media_playlist_response
                .text()
                .await
                .context("faild get playlist text content")?;
            /*
               MediaPlaylistの#EXT-X-TARGETDURATIONの秒数を超過するsegmentが含まれている時、そのままtry_fromでパースするとバリデーションでエラーになる
               allowable_excess_durationで超過を許容する分の秒数を設定することでバリデーションに引っかからないようにしている
               radikoのHLSでは#EXT-X-TARGETDURATIONが5秒なので、10秒までのsegmentを許容することにしている
               audio segmentをファイルにappendしていくことが目的なので、segmentあたりの秒数がTARGETDURATIONを超過していても関係がない
               hls_m3u8クレートのmedia_playlist.rsのテストコードtoo_large_segment_duration_testで確認できる
            */
            let media_play_list = MediaPlaylist::builder()
                .allowable_excess_duration(Duration::from_secs(5))
                .parse(media_playlist_content.as_str())
                .context("faild parse media playlist")?;
            let target_duration = media_play_list.target_duration;

            // 利用側でリトライ処理ができるようにエラーを伝搬させたい
            // しかし、無限ループでは関数の戻り値でエラーを伝搬できないのでチャネル経由でエラーも伝搬させる
            match self
                .handle_hls_stream_step(media_play_list, &mut last_processed_sequence)
                .await
            {
                Ok(audio_segments) => {
                    for audio_segment in audio_segments {
                        if tx.send(Ok(audio_segment)).await.is_err() {
                            // send自体がErrのときはReceiverに到達していない
                            break;
                        };
                    }
                }
                Err(e) => {
                    error!("handle hls stream step: {:#?}", e);
                    if tx.send(Err(e)).await.is_err() {
                        // send自体がErrのときはReceiverに到達していない
                        break Ok(());
                    };
                }
            }

            tokio::time::sleep(target_duration).await;
        }
    }

    async fn handle_hls_stream_step(
        &self,
        media_play_list: MediaPlaylist<'_>,
        last_processed_sequence: &mut usize,
    ) -> anyhow::Result<Vec<Bytes>> {
        let media_sequence = media_play_list.media_sequence;
        let segments = media_play_list.segments;

        let mut audio_segments = Vec::new();
        for (segment_sequence, segment) in segments {
            let segment_url = segment.uri();
            let processing_sequence = media_sequence + segment_sequence;
            if processing_sequence <= *last_processed_sequence {
                trace!(
                    "skip segment segment_sequence: {}, last_processed_sequence: {}, uri: {}",
                    segment_sequence, last_processed_sequence, &segment_url
                );
                continue;
            }

            let segment_response = self
                .inner
                .client
                .get(segment_url.to_string())
                .send()
                .await
                .context("failed get segment")?;
            let segment_bytes = segment_response
                .bytes()
                .await
                .context("failed get segment from response")?;
            // https://www.rfc-editor.org/rfc/rfc8216#section-3.4
            let packed_audio_segment = segment_bytes;
            let audio_bytes = Self::skip_id3_tag_bytes(packed_audio_segment)
                .await
                .context("failed skip id3 tag bytes")?;

            *last_processed_sequence = processing_sequence;
            audio_segments.push(audio_bytes);
            trace!(
                "process segment segment_sequence: {}, last_processed_sequence: {}, uri: {}",
                segment_sequence, last_processed_sequence, &segment_url
            );
        }

        Ok(audio_segments)
    }

    /// HLSで配信されるPacked AudioのID3ヘッダーをスキップして音声データを取り出す
    /// ID3: https://id3.org/id3v2.4.0-structure
    async fn skip_id3_tag_bytes(mut audio_segment: Bytes) -> anyhow::Result<Bytes> {
        let header_size = 10;
        let footer_size = 10;

        if audio_segment.len() < header_size {
            bail!("too short for ID3 header");
        }
        if &audio_segment[0..3] != b"ID3" {
            warn!("ID3 header not found");
            return Ok(audio_segment);
        }

        let _version_bytes = 3..5;
        let flags = &audio_segment[5];
        let size = (audio_segment[6] as u32) << 21
            | (audio_segment[7] as u32) << 14
            | (audio_segment[8] as u32) << 7
            | (audio_segment[9] as u32);
        let mut total_size = header_size + size as usize;
        // footerは固定で10バイト
        let has_footer = (flags & 0x10) != 0;
        if has_footer {
            total_size += footer_size;
        }

        audio_segment.advance(total_size);
        Ok(audio_segment)
    }
}

#[cfg(test)]
mod tests {

    use chrono::Utc;
    use chrono_tz::Asia::Tokyo;
    use reqwest::Client;
    use sanitise_file_name::sanitise;
    use tempfile::TempDir;

    use crate::{telemetry::init_telemetry, test_helper::radiko_client};

    use super::*;

    #[tokio::test]
    #[ignore = "ファイル録音処理が実行されて数秒を要するため"]
    async fn output_smoke() -> anyhow::Result<()> {
        init_telemetry("output_smoke_test", Some("trace"));
        let radiko_client = radiko_client().await;
        let now_on_air_programs = radiko_client.now_on_air_programs(None).await?;
        let program = now_on_air_programs.first().unwrap();
        let title = program.get_info();
        let media_list_url = radiko_client.media_list_url(&program.station_id).await?;
        let temp_dir = TempDir::with_prefix(format!(
            "test_segments_{}",
            Utc::now().with_timezone(&Tokyo).format("%Y%m%d")
        ))?;
        let stream_handler = Arc::new(StreamHandler::new(Client::new(), media_list_url));
        stream_handler
            .start_recording(
                temp_dir.path().to_path_buf(),
                &format!("{}.aac", sanitise(&title)),
                Duration::from_secs(6),
            )
            .await?;

        Ok(())
    }
}
