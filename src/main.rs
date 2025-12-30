use std::env;
use tdl::{fs::Dir, tidal::Access};

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

        let fs = Dir::new("/tmp/tdl-store")
            .await
            .with_context(|| "Failed to create or find the store.")?;

        let access = Access::log_in(&client_id, &client_secret, &streaming_tok).await?;

        let tracks = access.get_tracks("109100968").await?;
        eprintln!("{tracks:?}");

        for (n, ref track) in tracks.into_iter().enumerate() {
            let track = access.download_track(track, Some(n as u16)).await?;
            fs.add_music(track).await?;
        }

        Ok(())
    }
}
