use chrono::DateTime;
use chrono_tz::Tz;

const V2_URL: &str = "https://radiko.jp/v2/";
const V3_URL: &str = "https://radiko.jp/v3/";
const V4_URL: &str = "https://radiko.jp/v4/";
const API_URL: &str = "https://api.radiko.jp/";
const AREA_URL: &str = "https://radiko.jp/area/";

pub struct Endpoint {}

impl Endpoint {
    pub const RADIKO_HOST: &str = "https://radiko.jp";
    // radiko_session取得に利用
    pub const LOGIN_CHECK_URL: &str = "https://radiko.jp/ap/member/webapi/v2/member/login/check";

    pub const DATETIME_FORMAT: &str = "%Y%m%d%H%M%S";

    pub fn area_id_endpoint() -> String {
        AREA_URL.to_string()
    }

    pub fn login_endpoint() -> String {
        format!("{}api/member/login", V4_URL)
    }

    pub fn auth1_endpoint() -> String {
        format!("{}api/auth1", V2_URL)
    }

    pub fn auth2_endpoint() -> String {
        format!("{}api/auth2", V2_URL)
    }

    pub fn search_endpoint() -> String {
        format!("{}api/program/search", V3_URL)
    }

    pub fn station_list_from_area_id_endpoint(area_id: &str) -> String {
        format!("{}station/list/{}.xml", V3_URL, area_id)
    }

    pub fn station_list_all_endpoint() -> String {
        format!("{}station/region/full.xml", V3_URL)
    }

    // https://api.radiko.jp/program/v3/now/JP13.xml
    pub fn now_on_air_programs(area_id: &str) -> String {
        format!("{}program/v3/now/{}.xml", API_URL, area_id)
    }
    pub fn weekly_programs_endpoint(station_id: &str) -> String {
        format!("{}program/v3/weekly/{}.xml", API_URL, station_id)
    }

    #[allow(dead_code)]
    pub fn stream_url_list_endpoint(station_id: &str) -> String {
        format!("{}station/stream/pc_html5/{}.xml", V3_URL, station_id)
    }

    /// HLSストリーミングのMasterPlaylist.m3u8を返すエンドポイントを取得
    /// radikoによる仕様変更時にはエンドポイント自体が変更されたり、クエリパラメータが変更される模様
    pub fn playlist_create_url_endpoint(station_id: &str, lsid: &str) -> String {
        format!(
            "https://si-f-radiko.smartstream.ne.jp/so/playlist.m3u8?station_id={}&l=15&lsid={}&type=b",
            station_id, lsid
        )
    }

    pub fn area_free_playlist_create_url_endpoint(station_id: &str, lsid: &str) -> String {
        format!(
            "https://si-c-radiko.smartstream.ne.jp/so/playlist.m3u8?station_id={}&l=15&lsid={}&type=c",
            station_id, lsid
        )
    }

    /*
       example:
       host
           tf-f-rpaa-radiko.smartstream.ne.jp
       filename
           /tf/playlist.m3u8
       station_id
           LFR
       start_at
           20260425010000
       ft
           20260425010000
       end_at
           20260425030000
       to
           20260425030000
       seek
           20260425022823
       preroll
           0
       l
           15
       lsid
           318c1ca1e74a6d9ca55ee8eff65df857
       type
            b
    */
    /// 無料プランタイムフリーエンドポイント
    pub fn time_free_playlist_create_url_endpoint(
        station_id: &str,
        start_at: &DateTime<Tz>,
        end_at: &DateTime<Tz>,
        seek: &DateTime<Tz>,
        lsid: &str,
    ) -> String {
        let (start_at, end_at, seek) = (
            start_at.format(Self::DATETIME_FORMAT),
            end_at.format(Self::DATETIME_FORMAT),
            seek.format(Self::DATETIME_FORMAT),
        );
        format!(
            "https://tf-f-rpaa-radiko.smartstream.ne.jp/tf/playlist.m3u8?station_id={}&start_at={}&ft={}&end_at={}&to={}&seek={}&preroll=0&l=15&lsid={}&type=b",
            station_id, start_at, start_at, end_at, end_at, seek, lsid
        )
    }

    /// エリアフリープラン会員タイムフリーエンドポイント
    pub fn time_free_for_area_free_playlist_create_url_endpoint(
        station_id: &str,
        start_at: &DateTime<Tz>,
        end_at: &DateTime<Tz>,
        seek: &DateTime<Tz>,
        lsid: &str,
    ) -> String {
        let (start_at, end_at, seek) = (
            start_at.format(Self::DATETIME_FORMAT),
            end_at.format(Self::DATETIME_FORMAT),
            seek.format(Self::DATETIME_FORMAT),
        );
        format!(
            "https://tf-c-rpaa-radiko.smartstream.ne.jp/tf/playlist.m3u8?station_id={}&start_at={}&ft={}&end_at={}&to={}&seek={}&preroll=0&l=15&lsid={}&type=c",
            station_id, start_at, start_at, end_at, end_at, seek, lsid
        )
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, NaiveDateTime, TimeZone};
    use chrono_tz::{Asia::Tokyo, Tz};

    use super::Endpoint;
    use crate::{constants::test_constants::TEST_STATION_ID, radiko::api::utils::Utils};

    #[test]
    fn area_id_endpoint_test() {
        let get_area_id_endpoint = Endpoint::area_id_endpoint();
        assert_eq!(get_area_id_endpoint, "https://radiko.jp/area/");
    }

    #[test]
    fn auth1_endpoint_test() {
        let get_auth1_endpoint = Endpoint::auth1_endpoint();
        assert_eq!(get_auth1_endpoint, "https://radiko.jp/v2/api/auth1");
    }

    #[test]
    fn auth2_endpoint_test() {
        let get_auth2_endpoint = Endpoint::auth2_endpoint();
        assert_eq!(get_auth2_endpoint, "https://radiko.jp/v2/api/auth2");
    }

    #[test]
    fn stream_url_list_endpoint_test() {
        let get_stream_url_list_endpoint = Endpoint::stream_url_list_endpoint("TBS");
        assert_eq!(
            get_stream_url_list_endpoint,
            "https://radiko.jp/v3/station/stream/pc_html5/TBS.xml"
        );
    }

    #[test]
    fn search_endpoint_test() {
        assert_eq!(
            "https://radiko.jp/v3/api/program/search",
            Endpoint::search_endpoint()
        );
    }

    #[test]
    fn stations_list_from_area_id_endpoint() {
        let area_id = "JP13";
        assert_eq!(
            format!("https://radiko.jp/v3/station/list/{}.xml", area_id),
            Endpoint::station_list_from_area_id_endpoint(area_id)
        );
    }

    #[test]
    fn stations_list_all_endpoint() {
        assert_eq!(
            "https://radiko.jp/v3/station/region/full.xml",
            Endpoint::station_list_all_endpoint()
        );
    }

    #[test]
    fn now_on_air_programs() {
        let area_id = "JP13";
        assert_eq!(
            format!("https://api.radiko.jp/program/v3/now/{}.xml", area_id),
            Endpoint::now_on_air_programs(area_id)
        );
    }

    #[test]
    fn weekly_programs_endpoint() {
        let station_id = TEST_STATION_ID;
        assert_eq!(
            format!("https://api.radiko.jp/program/v3/weekly/{}.xml", station_id),
            Endpoint::weekly_programs_endpoint(station_id)
        );
    }

    #[test]
    fn playlist_create_url_endpoint_test() {
        let station_id = TEST_STATION_ID;
        let lsid = Utils::generate_md5_hash();
        let playlist_crate_url = Endpoint::playlist_create_url_endpoint(station_id, &lsid);
        assert_eq!(
            playlist_crate_url,
            format!(
                "https://si-f-radiko.smartstream.ne.jp/so/playlist.m3u8?station_id={}&l=15&lsid={}&type=b",
                station_id, lsid
            )
        )
    }

    #[test]
    fn area_free_playlist_create_url_endpoint_test() {
        let station_id = "MBS";
        let lsid = Utils::generate_md5_hash();
        let playlist_crate_url =
            Endpoint::area_free_playlist_create_url_endpoint(station_id, &lsid);
        assert_eq!(
            playlist_crate_url,
            format!(
                "https://si-c-radiko.smartstream.ne.jp/so/playlist.m3u8?station_id={}&l=15&lsid={}&type=c",
                station_id, lsid
            )
        )
    }

    fn datetime_parse_from_string(s: &str) -> DateTime<Tz> {
        Tokyo
            .from_local_datetime(
                &NaiveDateTime::parse_from_str(s, Endpoint::DATETIME_FORMAT).unwrap(),
            )
            .unwrap()
    }

    #[test]
    fn time_free_playlist_create_url_endpoint_test() {
        let station_id = "LFR";

        let start_at_s = "20260426010000";
        let end_at_s = "20260426030000";
        let seek_s = "20260426030000";
        let start_at = datetime_parse_from_string(start_at_s);
        let end_at = datetime_parse_from_string(end_at_s);
        let seek = datetime_parse_from_string(seek_s);
        let lsid = Utils::generate_md5_hash();
        let playlist_crate_url = Endpoint::time_free_playlist_create_url_endpoint(
            station_id, &start_at, &end_at, &seek, &lsid,
        );
        assert_eq!(
            playlist_crate_url,
            format!(
                "https://tf-f-rpaa-radiko.smartstream.ne.jp/tf/playlist.m3u8?station_id={}&start_at={}&ft={}&end_at={}&to={}&seek={}&preroll=0&l=15&lsid={}&type=b",
                station_id, start_at_s, start_at_s, end_at_s, end_at_s, seek_s, lsid
            )
        )
    }

    #[test]
    fn time_free_for_area_free_playlist_create_url_endpoint_test() {
        let station_id = "LFR";
        let start_at_s = "20260426010000";
        let end_at_s = "20260426030000";
        let seek_s = "20260426030000";
        let start_at = datetime_parse_from_string(start_at_s);
        let end_at = datetime_parse_from_string(end_at_s);
        let seek = datetime_parse_from_string(seek_s);
        let lsid = Utils::generate_md5_hash();
        let playlist_crate_url = Endpoint::time_free_for_area_free_playlist_create_url_endpoint(
            station_id, &start_at, &end_at, &seek, &lsid,
        );
        assert_eq!(
            playlist_crate_url,
            format!(
                "https://tf-c-rpaa-radiko.smartstream.ne.jp/tf/playlist.m3u8?station_id={}&start_at={}&ft={}&end_at={}&to={}&seek={}&preroll=0&l=15&lsid={}&type=c",
                station_id, start_at_s, start_at_s, end_at_s, end_at_s, seek_s, lsid
            )
        )
    }
}
