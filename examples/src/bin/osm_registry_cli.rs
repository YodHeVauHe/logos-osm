//! Auto-generated instruction CLI over the committed IDL: builds and
//! submits `init-registry`, `register-region` and `register-batch`
//! transactions (see `spel::run`). The SDK's SpelCliRegistryClient shells
//! out to this binary.
#[tokio::main]
async fn main() {
    spel::run().await;
}
