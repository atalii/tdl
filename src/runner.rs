use crate::{fs::Dir, tidal::Access};
use std::{env, path::PathBuf, str::FromStr};

use anyhow::{Context, Result, anyhow, bail};

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

        let api_client_id =
            env::var("TDL_API_CLIENT_ID").with_context(|| "Failed to find $TDL_API_CLIENT_ID")?;
        let api_client_secret = env::var("TDL_API_CLIENT_SECRET")
            .with_context(|| "Failed to find $TDL_API_CLIENT_SECRET")?;

        let api = Access::log_in(
            &api_client_id,
            &api_client_secret,
            &client_id,
            &client_secret,
        )
        .await?;

        let fs = Dir::new("/tmp/tdl-store")
            .await
            .with_context(|| "Failed to create or find the store")?;

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

    pub async fn repl(&self) -> Result<()> {
        use rustyline::{DefaultEditor, error::ReadlineError};
        let mut rl = DefaultEditor::new()?;

        loop {
            match rl.readline("𝄞 ") {
                Ok(x) => {
                    if let Err(e) = self.run_cmd(&x).await {
                        eprintln!("{:?}", e);
                    }
                }
                Err(ReadlineError::Eof) => break,
                Err(ReadlineError::Interrupted) => (),
                Err(e) => eprintln!("{:?}", e),
            }
        }

        eprintln!("goodbye :)!");
        Ok(())
    }

    async fn run_cmd(&self, cmdline: &str) -> Result<()> {
        let mut cmdline = cmdline.split_whitespace();
        let cmd = cmdline.next().ok_or(anyhow!("No command supplied."))?;
        match cmd {
            "track" => {
                let target = cmdline.next().ok_or(anyhow!("No target supplied."))?;
                let index = match cmdline.next() {
                    Some(x) => Some(u16::from_str(x)?),
                    None => None,
                };

                self.fetch_track(target, index).await?;
            }
            "album" => {
                let target = cmdline.next().ok_or(anyhow!("No target supplied."))?;
                self.fetch_album(target).await?;
            }
            x => bail!("Unknown command: {x}"),
        }
        Ok(())
    }
}
