use jiff::Zoned;
use tracing::{error, info};

use crate::RADYKO_TZ_NAME;

pub struct Utils {}
impl Utils {
    pub fn now_in_tz_tokyo() -> Zoned {
        Zoned::now()
            .datetime()
            .in_tz(RADYKO_TZ_NAME)
            .map_err(|e| error!("failed Zoned::now() in_tz {RADYKO_TZ_NAME} error: {e:#?}"))
            .unwrap()
    }

    pub fn formated_now_in_tz_tokyo() -> anyhow::Result<String> {
        Ok(Self::now_in_tz_tokyo()
            .strftime("%Y-%m-%d %H:%M:%S")
            .to_string())
    }

    /// 録音出力ディレクトリに書き込み可能かをチェック
    pub fn is_writable_output_dir(output_dir: &str) {
        std::fs::create_dir_all(output_dir)
            .unwrap_or_else(|_| panic!("failed create directory: {:#?}", output_dir));
        tempfile::tempfile_in(output_dir).expect("failed create test_file in output_dir");
        info!("output_dir is writable");
    }
}

#[cfg(test)]
mod utils_tests {
    use jiff::civil::DateTime;

    use crate::{RADYKO_TZ_NAME, radiko::api::endpoint::Endpoint};

    #[test]
    fn jiff_play_ground() {
        let datetime_s = "20260426010000";
        let datetime = DateTime::strptime(Endpoint::DATETIME_FORMAT, datetime_s)
            .unwrap()
            .in_tz(RADYKO_TZ_NAME)
            .unwrap();

        assert_eq!(
            datetime_s,
            datetime.strftime(Endpoint::DATETIME_FORMAT).to_string()
        )
    }
}
