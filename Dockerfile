# https://kerkour.com/rust-production-checklist
# docker build -t radyko:alpine . && docker run -ti --rm radyko:alpine search --keyword "オールナイトニッポン" --station-id "LFR"
# https://hub.docker.com/_/rust/#rustversion-alpine
# rust:alpineイメージ上でcargo build --releaseとしているのでmusl libcによる静的リンクビルドになる
####################################################################################################
## Build
####################################################################################################
FROM rust:alpine AS build

RUN apk update && \
    apk upgrade --no-cache && \
    apk add --no-cache mold musl musl-dev libc-dev cmake clang clang-dev file \
        git make build-base bash curl wget zip gnupg coreutils gcc g++  zstd binutils ca-certificates upx

# 先に空のプロジェクトを作成して依存関係のビルドのみを済ませておいてキャッシュする
RUN cargo new --bin radyko
WORKDIR /radyko
COPY Cargo.toml Cargo.lock .cargo/ ./
RUN cargo build --release && rm src/*.rs

# 自分のソースコードをコピーしてビルドする
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release


####################################################################################################
## This stage is used to get the correct files into the final image
####################################################################################################
FROM alpine:latest AS files

ENV TZ=Asia/Tokyo
ENV PUID=1000
ENV PGID=1000
ENV APP_USER=radyko
ENV APP_GROUP=radyko

# mailcap is used for content type (MIME type) detection
# tzdata is used for timezone info
RUN apk update && \
    apk upgrade --no-cache && \
    apk add --no-cache ca-certificates mailcap tzdata && \
    cp /usr/share/zoneinfo/Asia/Tokyo /etc/localtime && \
    # タイムゾーンを日本に固定
    echo "${TZ}" > /etc/timezone

RUN update-ca-certificates

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${PUID}" \
    "${APP_USER}" && \
    mkdir -p /app && \
    chown -R "${PUID}:${PGID}" /app && \
    chown 755 /app


####################################################################################################
## Final image
####################################################################################################
FROM scratch

ENV TZ=Asia/Tokyo
ENV PUID=1000
ENV PGID=1000
ENV APP_USER=radyko
ENV APP_GROUP=radyko

# ここで明示的に/etcを--chmod=555としておかないと、次のCOPY --chmod=444により/etcに実行権限が付与されないことから探索できず、Permission Deniedエラーとなる
# プログラム(radyko)で利用しているクレート(hickory-dns)が/etc/resolv.confを見に行くので、/etcを探索できる必要がある
# https://docs.rs/hickory-resolver/0.26.0/hickory_resolver/system_conf/
COPY --from=files --chmod=555 /etc /etc

# /etc/nsswitch.conf may be used by some DNS resolvers
# /etc/mime.types may be used to detect the MIME type of files
COPY --from=files --chmod=444 \
    /etc/passwd \
    /etc/group \
    /etc/nsswitch.conf \
    /etc/mime.types \
    /etc/

COPY --from=files --chmod=444 /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=files --chmod=444 /etc/localtime /etc/localtime
COPY --from=files --chmod=444 /etc/timezone /etc/timezone
COPY --from=files --chmod=444 /usr/share/zoneinfo /usr/share/zoneinfo

# アプリケーションが作業するディレクトリのオーナーを明示的に指定してコピーすることで実行時に権限周りのエラーが発生しないようにしている
COPY --from=files --chown="${PUID}:${PUID}" /app /app

# 実行バイナリのコピー
COPY --from=build /radyko/target/release/radyko /bin/radyko

USER "${PUID}:${PGID}"

WORKDIR /app

ENTRYPOINT ["/bin/radyko"]
