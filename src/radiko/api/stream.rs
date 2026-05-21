use std::{borrow::Cow, convert::TryFrom, io::Write, sync::Arc};

use anyhow::{Context, anyhow, bail};
use futures::{Stream, StreamExt};
use hls_m3u8::MasterPlaylist;
use tempfile::NamedTempFile;

use crate::{
    RADYKO_CONCURRENCY,
    model::program::program_id::{ProgramId, SeekStartAt},
};

use super::{auth::RadikoAuth, endpoint::Endpoint};

#[derive(Debug, Clone)]
pub struct RadikoStream {
    inner: Arc<RadikoStreamRef>,
}

#[derive(Debug)]
struct RadikoStreamRef {
    radiko_auth: RadikoAuth,
}

impl RadikoStream {
    pub fn new(radiko_auth: RadikoAuth) -> Self {
        Self {
            inner: Arc::new(RadikoStreamRef { radiko_auth }),
        }
    }

    pub fn live_stream_url(&self, station_id: &str) -> String {
        let lsid = &self.inner.radiko_auth.lsid().to_string();
        if self.inner.radiko_auth.area_free() {
            Endpoint::area_free_playlist_create_url_endpoint(station_id, lsid)
        } else {
            Endpoint::playlist_create_url_endpoint(station_id, lsid)
        }
    }

    pub async fn get_medialist_url_for_live(
        &self,
        station_id: &str,
    ) -> anyhow::Result<Cow<'_, str>> {
        let master_playlist_content: &str = &self
            .get_hls_master_playlist_content_for_live(station_id)
            .await?;
        let Ok(master_playlist) = MasterPlaylist::try_from(master_playlist_content) else {
            bail!("master_playlist_content: {:#?}", master_playlist_content)
        };

        Ok(master_playlist
            .variant_streams
            .first()
            .and_then(|variant_stream| match variant_stream {
                hls_m3u8::tags::VariantStream::ExtXStreamInf { uri, .. } => Some(uri.to_string()),
                _ => None,
            })
            .with_context(|| {
                format!(
                    "failed load medialist url MasterPlaylist Content: {:#?}",
                    master_playlist
                )
            })?
            .into())
    }

    /// medialist urlからタイムフリー音声配信URLを非同期に取得する
    pub fn stream_timefree_medialist_urls(
        &self,
        program_id: ProgramId,
    ) -> impl Stream<Item = anyhow::Result<String>> {
        let seek_times = SeekStartAt::calculate_seek_start_times(
            program_id.start_at().clone(),
            program_id.end_at().clone(),
        );
        futures::stream::iter(seek_times)
            .map(move |seek_time| {
                let this = self.clone();
                let program_id = program_id.clone();
                async move {
                    // ここでセッション付きの音声配信エンドポイントURLが取得できるがセッションの有効期間が短い（具体的な期間までは未検証）
                    // 音声配信エンドポイントURLを一括で取得して後続の処理を行うと、処理の途中でセッション切れになってしまい配信エンドポイントURLが無効になる現象に遭遇した
                    this.get_medialist_url_for_timefree(program_id, seek_time)
                        .await
                }
            })
            .buffer_unordered(RADYKO_CONCURRENCY)
    }

    pub async fn download_playlist_to_tempfile(
        &self,
        station_id: &str,
    ) -> anyhow::Result<NamedTempFile> {
        let playlist_content = self
            .inner
            .radiko_auth
            .http_client()
            .get(self.live_stream_url(station_id))
            .send()
            .await?
            .bytes()
            .await?;

        let mut temp_file = NamedTempFile::with_suffix(".m3u8")?;
        temp_file.write_all(&playlist_content)?;
        temp_file.flush()?;

        Ok(temp_file)
    }

    pub async fn get_medialist_url_for_timefree(
        &self,
        program_id: ProgramId,
        seek_start_at: SeekStartAt,
    ) -> anyhow::Result<String> {
        let master_playlist_res = self
            .inner
            .radiko_auth
            .http_client()
            .get(self.timefree_stream_url(program_id, seek_start_at))
            .send()
            .await?;

        if !master_playlist_res.status().is_success() {
            return Err(anyhow!(
                "get hls master playlist error: {:#?}, client_info: {:#?}",
                master_playlist_res.text().await?,
                self.inner.radiko_auth.http_client()
            ));
        }

        let master_playlist_content: &str = &master_playlist_res.text().await?;
        let Ok(master_playlist) = MasterPlaylist::try_from(master_playlist_content) else {
            bail!("master_playlist_content: {:#?}", master_playlist_content)
        };

        master_playlist
            .variant_streams
            .first()
            .and_then(|variant_stream| match variant_stream {
                hls_m3u8::tags::VariantStream::ExtXStreamInf { uri, .. } => Some(uri.to_string()),
                _ => None,
            })
            .with_context(|| {
                format!(
                    "failed load medialist url MasterPlaylist Content: {:#?}",
                    master_playlist
                )
            })
    }

    async fn get_hls_master_playlist_content_for_live(
        &self,
        station_id: &str,
    ) -> anyhow::Result<Cow<'_, str>> {
        let master_playlist_res = self
            .inner
            .radiko_auth
            .http_client()
            .get(self.live_stream_url(station_id))
            .send()
            .await?;

        if !master_playlist_res.status().is_success() {
            return Err(anyhow!(
                "get hls master playlist error: {:#?}, client_info: {:#?}",
                master_playlist_res.text().await?,
                self.inner.radiko_auth.http_client()
            ));
        }

        Ok(master_playlist_res.text().await?.into())
    }

    fn timefree_stream_url(&self, program_id: ProgramId, seek_start_at: SeekStartAt) -> String {
        let lsid = &self.inner.radiko_auth.lsid().to_string();
        if self.inner.radiko_auth.area_free() {
            Endpoint::timefree_for_area_free_playlist_create_url_endpoint(
                &program_id,
                &seek_start_at,
                lsid,
            )
        } else {
            Endpoint::timefree_playlist_create_url_endpoint(program_id, &seek_start_at, lsid)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Seek},
        ops::Not,
    };

    use crate::{
        constants::test_constants::TEST_STATION_ID,
        radiko::test_helper::{AuthType, radiko_stream},
    };

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn area_free_radiko_stream_smoke() -> anyhow::Result<()> {
        let radiko_stream = radiko_stream(AuthType::AreaFree).await;

        let station_id = TEST_STATION_ID;
        let hls_master_playlist_content = radiko_stream
            .get_hls_master_playlist_content_for_live(station_id)
            .await?;
        assert!(hls_master_playlist_content.is_empty().not());
        assert!(
            radiko_stream
                .get_medialist_url_for_live(station_id)
                .await?
                .is_empty()
                .not()
        );
        assert!(radiko_stream.live_stream_url(station_id).is_empty().not());

        let mut playlist_file = radiko_stream
            .download_playlist_to_tempfile(station_id)
            .await?;
        let mut buf = String::new();
        playlist_file.seek(std::io::SeekFrom::Start(0))?;
        playlist_file.as_file().read_to_string(&mut buf)?;
        assert!(buf.is_empty().not());

        Ok(())
    }

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn not_area_free_radiko_stream_smoke() -> anyhow::Result<()> {
        let radiko_stream = radiko_stream(AuthType::Normal).await;

        let hls_master_playlist_content = radiko_stream
            .get_hls_master_playlist_content_for_live(TEST_STATION_ID)
            .await?;
        assert!(hls_master_playlist_content.is_empty().not());
        assert!(
            radiko_stream
                .get_medialist_url_for_live(TEST_STATION_ID)
                .await?
                .is_empty()
                .not()
        );
        assert!(
            radiko_stream
                .live_stream_url(TEST_STATION_ID)
                .is_empty()
                .not()
        );

        let mut playlist_file = radiko_stream
            .download_playlist_to_tempfile(TEST_STATION_ID)
            .await?;
        let mut buf = String::new();
        playlist_file.seek(std::io::SeekFrom::Start(0))?;
        playlist_file.as_file().read_to_string(&mut buf)?;
        assert!(buf.is_empty().not());

        Ok(())
    }
}
