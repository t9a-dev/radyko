use chrono::Utc;
use md5::{Digest, Md5};
use rand::RngExt;

pub struct Utils {}
impl Utils {
    /// 適当なlsidっぽい値を生成することを目的としている。
    pub fn generate_md5_hash() -> String {
        // 0から1000000000の間のランダムな整数を生成
        let mut rng = rand::rng();
        let random_num: u32 = rng.random_range(0..1000000000);

        // 現在時刻をミリ秒で取得
        let now = Utc::now();
        let timestamp = now.timestamp_millis();

        // 文字列として連結
        let input_string = format!("{}{}", random_num, timestamp);

        // MD5ハッシュを計算
        let mut hasher = Md5::new();
        hasher.update(input_string.as_bytes());
        let result = hasher.finalize();

        // 16進数文字列として返す
        hex::encode(result).to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Not;

    use super::*;

    #[test]
    fn generate_md5_hash_smoke() {
        let dummy_md5_hash = Utils::generate_md5_hash();
        assert!(dummy_md5_hash.is_empty().not());
        assert_eq!(dummy_md5_hash.len(), 32);
    }
}
