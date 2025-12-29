use std::env;
use tdl::tidal::Access;

use anyhow::{Context, Result};

#[tokio::main]
async fn main() {
    if let Err(e) = inner().await {
        eprintln!("\x1b[31;1mA fatal error occurred:\x1b[0m {e:?}");
        std::process::exit(1);
    }

    async fn inner() -> Result<()> {
        let client_id =
            env::var("TDL_CLIENT_ID").with_context(|| "Failed to find $TDL_CLIENT_ID")?;
        let client_secret =
            env::var("TDL_CLIENT_SECRET").with_context(|| "Failed to find $TDL_CLIENT_SECRET")?;
        let streaming_tok = env::var("TDL_BEARER_STREAMING")
            .with_context(|| "Failed to find $TDL_BEARER_STREAMING")?;

        let access = Access::log_in(&client_id, &client_secret, &streaming_tok).await?;
        access.download_track("441696040").await?;

        Ok(())
    }
}
