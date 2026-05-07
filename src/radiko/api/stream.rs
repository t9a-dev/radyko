use std::{borrow::Cow, convert::TryFrom, io::Write, sync::Arc};

use anyhow::{Context, anyhow, bail};
use chrono::{DateTime, TimeDelta};
use chrono_tz::Tz;
use futures::{StreamExt, TryStreamExt};
use hls_m3u8::MasterPlaylist;
use tempfile::NamedTempFile;
use tracing::error;

use crate::RADYKO_CONCURRENCY;

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
            })
            .unwrap()
            .into())
    }

    pub async fn collect_timefree_medialist_urls(
        &self,
        station_id: String,
        start_at: DateTime<Tz>,
        end_at: DateTime<Tz>,
    ) -> anyhow::Result<Vec<String>> {
        let seek_times = Self::calculate_seek_start_times(start_at, end_at);
        let medialist_urls = futures::stream::iter(seek_times)
            .map(|seek_time| {
                let this = self.clone();
                let station_id = station_id.clone();
                async move {
                    this.get_medialist_url_for_timefree(station_id, start_at, end_at, seek_time)
                        .await
                }
            })
            .buffer_unordered(RADYKO_CONCURRENCY)
            .try_collect::<Vec<_>>()
            .await?;

        Ok(medialist_urls)
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

    fn timefree_stream_url(
        &self,
        station_id: String,
        start_at: DateTime<Tz>,
        end_at: DateTime<Tz>,
        seek: DateTime<Tz>,
    ) -> String {
        let lsid = &self.inner.radiko_auth.lsid().to_string();
        if self.inner.radiko_auth.area_free() {
            Endpoint::timefree_for_area_free_playlist_create_url_endpoint(
                &station_id,
                &start_at,
                &end_at,
                &seek,
                lsid,
            )
        } else {
            Endpoint::timefree_playlist_create_url_endpoint(
                &station_id,
                &start_at,
                &end_at,
                &seek,
                lsid,
            )
        }
    }

    async fn get_medialist_url_for_timefree(
        &self,
        station_id: String,
        start_at: DateTime<Tz>,
        end_at: DateTime<Tz>,
        seek: DateTime<Tz>,
    ) -> anyhow::Result<String> {
        let master_playlist_res = self
            .inner
            .radiko_auth
            .http_client()
            .get(self.timefree_stream_url(station_id, start_at, end_at, seek))
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

    fn calculate_seek_start_times(
        mut start_at: DateTime<Tz>,
        end_at: DateTime<Tz>,
    ) -> Vec<DateTime<Tz>> {
        if end_at <= start_at {
            error!("end must be greater than start");
            return vec![];
        }

        let mut times = vec![];
        while start_at < end_at {
            times.push(start_at);
            let Some(next_time) = start_at.checked_add_signed(TimeDelta::seconds(15)) else {
                break;
            };
            start_at = next_time;
        }

        times
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Seek},
        ops::Not,
    };

    use chrono::{NaiveDateTime, TimeZone};
    use chrono_tz::Asia::Tokyo;

    use crate::{
        constants::test_constants::TEST_STATION_ID,
        radiko::{
            api::{endpoint::Endpoint, stream::RadikoStream},
            test_helper::{AuthType, radiko_stream},
        },
    };

    #[tokio::test]
    #[ignore = "エリアフリー会員情報を持つことに依存しているテスト"]
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

    #[test]
    fn calculate_seek_start_times_test() {
        let start = Tokyo
            .from_local_datetime(
                &NaiveDateTime::parse_from_str("2000-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
            )
            .unwrap();
        let end = Tokyo
            .from_local_datetime(
                &NaiveDateTime::parse_from_str("2000-01-01 00:01:00", "%Y-%m-%d %H:%M:%S").unwrap(),
            )
            .unwrap();
        let mut seek_start_times = RadikoStream::calculate_seek_start_times(start, end);
        seek_start_times.sort();

        assert_eq!(
            seek_start_times[0]
                .format(Endpoint::DATETIME_FORMAT)
                .to_string(),
            "20000101000000"
        );
        assert_eq!(
            seek_start_times[1]
                .format(Endpoint::DATETIME_FORMAT)
                .to_string(),
            "20000101000015"
        );
        assert_eq!(
            seek_start_times[2]
                .format(Endpoint::DATETIME_FORMAT)
                .to_string(),
            "20000101000030"
        );
        assert_eq!(
            seek_start_times[3]
                .format(Endpoint::DATETIME_FORMAT)
                .to_string(),
            "20000101000045".to_string()
        );

        assert_eq!(seek_start_times.len(), 4);
    }
}
