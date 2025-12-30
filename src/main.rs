use anyhow::Result;
use tdl::runner;

#[tokio::main]
async fn main() {
    if let Err(e) = inner().await {
        eprintln!("\x1b[31;1mA fatal error occurred:\x1b[0m {e:?}");
        std::process::exit(1);
    }

    async fn inner() -> Result<()> {
        let runner = runner::Runner::new().await?;
        runner.fetch_album("422567306").await?;

        Ok(())
    }
}
