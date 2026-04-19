use chrono::{DateTime, Utc};
use chrono_tz::{Asia::Tokyo, Tz};
use tracing::{error, info};

use std::time::Duration;

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

    pub async fn retry_with_backoff<F, Fut, T, E>(
        mut f: F,
        max_attempts: u32,
        base_delay: Duration,
        max_delay: Duration,
    ) -> anyhow::Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<T, E>>,
        E: std::fmt::Debug,
    {
        let mut attempt = 0;

        loop {
            match f().await {
                Ok(v) => return Ok(v),
                Err(e) => {
                    attempt += 1;

                    if attempt >= max_attempts {
                        error!("reached max attempts error: {:#?}", e);
                        return Err(e);
                    }

                    // delay = min(max_delay, base * 2^attempt)
                    let delay = {
                        let exp = 2u64.saturating_pow(attempt);
                        let base_ms = base_delay.as_millis() as u64;
                        let max_ms = max_delay.as_millis() as u64;

                        let delay_ms = base_ms.saturating_mul(exp).min(max_ms);
                        Duration::from_millis(delay_ms)
                    };

                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
}
