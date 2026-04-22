use chrono::{DateTime, Utc};
use chrono_tz::{Asia::Tokyo, Tz};
use tracing::info;

pub struct Utils {}
impl Utils {
    pub fn now_with_timezone_tokyo() -> DateTime<Tz> {
        Utc::now().with_timezone(&Tokyo)
    }

    pub fn formated_now_with_timezone_tokyo() -> String {
        Self::now_with_timezone_tokyo()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
    }

    /// 録音出力ディレクトリに書き込み可能かをチェック
    pub fn is_writable_output_dir(output_dir: &str) {
        std::fs::create_dir_all(output_dir).unwrap();
        tempfile::tempfile_in(output_dir).unwrap();
        info!("output_dir is writable");
    }
}
