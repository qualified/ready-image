use kube::Client;
use tracing_subscriber::fmt::format::FmtSpan;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    let sleeper_image = std::env::var("SLEEPER_IMAGE").expect("SLEEPER_IMAGE must be set");
    if sleeper_image.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "SLEEPER_IMAGE must not be empty",
        )
        .into());
    }

    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info,ready_image=trace".to_owned());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_span_events(FmtSpan::CLOSE)
        .init();

    let client = Client::try_default().await?;
    ready_image::run(client, sleeper_image).await;
    Ok(())
}
