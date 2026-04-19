use radyko::cli;

use mimalloc::MiMalloc;

// 静的リンクでビルド(XXX-unknown-linux-musl)するときに利用されるメモリアロケータ(malloc)より効率的なmimallocを利用する
// https://kerkour.com/rust-docker-small-secure-images
// Dockerfileのbuilder stageでrust:alpineを利用しており、musl libcを利用した静的リンクのビルドが行われる
// https://hub.docker.com/_/rust/#rustversion-alpine
// https://blog.rust-jp.rs/tatsuya6502/posts/2019-12-statically-linked-binary/
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if let Err(err) = cli::run().await {
        eprintln!("error: {:#?}", err);
        std::process::exit(1);
    }
    Ok(())
}
