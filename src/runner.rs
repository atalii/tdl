use crate::{fs::Dir, tidal::Access};
use std::{env, path::PathBuf};

use anyhow::{Context, Result};

pub struct Runner {
    fs: Dir,
    api: Access,
}

impl Runner {
    pub async fn new() -> Result<Self> {
        let client_id =
            env::var("TDL_CLIENT_ID").with_context(|| "Failed to find $TDL_CLIENT_ID")?;
        let client_secret =
            env::var("TDL_CLIENT_SECRET").with_context(|| "Failed to find $TDL_CLIENT_SECRET")?;
        let streaming_tok = env::var("TDL_BEARER_STREAMING")
            .with_context(|| "Failed to find $TDL_BEARER_STREAMING")?;

        let fs = Dir::new("/tmp/tdl-store")
            .await
            .with_context(|| "Failed to create or find the store")?;

        let api = Access::log_in(&client_id, &client_secret, &streaming_tok).await?;
        Ok(Self { fs, api })
    }

    pub async fn fetch_track<T: AsRef<str>>(&self, track: T, num: Option<u16>) -> Result<PathBuf> {
        let track = self
            .api
            .download_track(&track, num)
            .await
            .with_context(|| format!("Failed to downnload track: {}", track.as_ref()))?;

        self.fs
            .add_music(&track)
            .await
            .with_context(|| format!("Failed to save track to: {}", &track.display()))?;

        Ok(track)
    }

    pub async fn fetch_album<T: AsRef<str>>(&self, album: T) -> Result<()> {
        let tracks = self
            .api
            .get_tracks(album.as_ref())
            .await
            .with_context(|| format!("Failed to find tracks in album: {}", album.as_ref()))?;

        for (n, ref track) in tracks.into_iter().enumerate() {
            let track = self
                .fetch_track(track, Some((n + 1) as u16))
                .await
                .with_context(|| format!("Failed to download album: {}", album.as_ref()))?;

            self.fs
                .add_music(track)
                .await
                .with_context(|| format!("Failed to save album: {}", album.as_ref()))?;
        }

        Ok(())
    }
}
