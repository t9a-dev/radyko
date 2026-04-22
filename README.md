# radyko(らでぃこ)
`radyko`はインターネットラジオサービスradikoの非公式ツール（番組録音・検索）です。HLS(HTTP Live Streaming)録音処理を実装しているため`FFmpeg`を必要としません。  
エリアフリープラン契約ユーザーの利用を想定しています。無料プランユーザーの利用も可能ですがテストを行っていません。radiko地域判定と定義したルールの齟齬により正常に動作しない可能性があります。

## 使い方
### 共通
- `git clone https://github.com/t9a-dev/radyko.git`
- `cd radyko`
- `sudo chmod +x ./init.sh && ./init.sh`実行後 `.env`ファイルの中身をradikoエリアフリーに登録しているメールアドレスとパスワードで置き換えます。エリアフリー未登録の場合は`RADIKO_AREA_FREE_*`の環境変数を削除してください。
- `docker run ghcr.io/t9a-dev/radyko:latest init > radyko.toml`で設定ファイルを初期化します。
  - `radyko.toml`を編集して録音したい番組のキーワードやスケジュール(cron)を指定します。

#### Docker Compose
- `docker compose up -d`実行で録音サーバーが立ち上がります。
- `docker compose logs recorder`実行でログを確認できます。

#### Docker
- `docker run -v $(pwd)/radyko.toml:/app/radyko.toml -v $(pwd)/recorded:/app/recorded --env-file .env ghcr.io/t9a-dev/radyko:latest recorder -c radyko.toml`で`radyko.toml`で定義されているルールに一致する番組を録音します。
  - `cron`式によるスケジュール又はキーワードによるルール定義に対応しています。
  - `docker run -v $(pwd)/radyko.toml:/app/radyko.toml ghcr.io/t9a-dev/radyko:latest rule -c radyko.toml`で設定ファイルの定義ルールに一致する番組一覧を表示します。
- `docker run ghcr.io/t9a-dev/radyko:latest search -k "オールナイトニッポン" -s "LFR"`でキーワード`オールナイトニッポン`、放送局ID`LFR`に一致する番組を検索して結果を表示します。

#### Cargo
`cargo run -- search -k "オールナイトニッポン" -s "LFR"`のように実行します。

##### build
`cargo build --release`でビルドを実行し、`target/release/radyko`を利用します。

## 開発メモ
### `ignore`属性テストについて
radikoのWeb APIに依存しているテストは基本的に`ignore`属性を付与しています。radikoのWeb APIは日本国外のIPアドレスをブロックします。GitHub Actionsのランナー上でradikoにアクセスするテストを実行すると必ず失敗するといったように環境に依存しているためです。  
`cargo test -- --include-ignored`で`ignore`属性を含めた全てのテストを実行できます。

### instaによるスナップショットテスト
[`insta`](https://insta.rs/docs/quickstart/)によるスナップショットテストを設定ファイルのパース処理テストで利用しています。設定ファイルのパース処理を変更した場合、`cargo insta review`によりスナップショット確認と更新を行います。

## 注意事項
- 本ツールを利用して取得したコンテンツの著作権は各権利者に帰属します。録音データは私的利用の範囲内でのみ利用してください。  
- 本ツールの利用により発生したいかなる損害についても作者は責任を負いません。各自の責任において利用してください。  
- `radiko`は株式会社radikoの登録商標です。本ツールは非公式であり、同社とは一切関係ありません。

## ライセンス

本プロジェクトは、以下のいずれかのライセンスのもとで利用できます。

* MIT License（[LICENSE-MIT](LICENSE-MIT)）
* Apache License 2.0（[LICENSE-APACHE](LICENSE-APACHE)）

利用者は上記のいずれかを選択して適用できます。
