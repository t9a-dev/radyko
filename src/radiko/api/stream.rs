use std::{borrow::Cow, convert::TryFrom, io::Write, sync::Arc};

use anyhow::{Context, anyhow, bail};
use hls_m3u8::MasterPlaylist;
use tempfile::NamedTempFile;

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

    pub fn stream_url(&self, station_id: &str) -> String {
        let lsid = &self.inner.radiko_auth.lsid().to_string();
        if self.inner.radiko_auth.area_free() {
            Endpoint::area_free_playlist_create_url_endpoint(station_id, lsid)
        } else {
            Endpoint::playlist_create_url_endpoint(station_id, lsid)
        }
    }

    pub async fn get_hls_master_playlist_content(
        &self,
        station_id: &str,
    ) -> anyhow::Result<Cow<'_, str>> {
        let master_playlist_res = self
            .inner
            .radiko_auth
            .http_client()
            .get(self.stream_url(station_id))
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

    pub async fn download_playlist_to_tempfile(
        &self,
        station_id: &str,
    ) -> anyhow::Result<NamedTempFile> {
        let playlist_content = self
            .inner
            .radiko_auth
            .http_client()
            .get(self.stream_url(station_id))
            .send()
            .await?
            .bytes()
            .await?;

        let mut temp_file = NamedTempFile::with_suffix(".m3u8")?;
        temp_file.write_all(&playlist_content)?;
        temp_file.flush()?;

        Ok(temp_file)
    }

    pub async fn get_medialist_url(&self, station_id: &str) -> anyhow::Result<Cow<'_, str>> {
        let master_playlist_content: &str =
            &self.get_hls_master_playlist_content(station_id).await?;
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
            })
            .unwrap()
            .into())
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
    #[ignore = "エリアフリー会員情報を持つことに依存しているテスト"]
    async fn area_free_radiko_stream_smoke() -> anyhow::Result<()> {
        let radiko_stream = radiko_stream(AuthType::AreaFree).await;

        let station_id = TEST_STATION_ID;
        let hls_master_playlist_content = radiko_stream
            .get_hls_master_playlist_content(station_id)
            .await?;
        assert!(hls_master_playlist_content.is_empty().not());
        assert!(
            radiko_stream
                .get_medialist_url(station_id)
                .await?
                .is_empty()
                .not()
        );
        assert!(radiko_stream.stream_url(station_id).is_empty().not());

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
            .get_hls_master_playlist_content(TEST_STATION_ID)
            .await?;
        assert!(hls_master_playlist_content.is_empty().not());
        assert!(
            radiko_stream
                .get_medialist_url(TEST_STATION_ID)
                .await?
                .is_empty()
                .not()
        );
        assert!(radiko_stream.stream_url(TEST_STATION_ID).is_empty().not());

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
