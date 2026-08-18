use crate::{fs::Dir, tidal::Access};
use std::{env, path::PathBuf, str::FromStr, sync::Arc, time::Duration};

use anyhow::{Context, Result, anyhow, bail};
use tokio::sync::Mutex;

pub struct Runner {
    fs: Dir,
    api: Arc<Mutex<Access>>,
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
        let api = Arc::new(Mutex::new(api));
        Ok(Self { fs, api })
    }

    pub async fn run_refresh(&self) -> Result<()> {
        let mut refresh_token = env::var("TDL_CLIENT_REFRESH_STREAMING")
            .with_context(|| "Failed to find $TDL_CLIENT_REFRESH_STREAMING")?
            .to_string();
        let client_id = env::var("TDL_CLIENT_ID_STREAMING")
            .with_context(|| "Failed to find $TDL_CLIENT_ID_STREAMING")?
            .to_string();
        let api_lock = self.api.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_hours(1)).await;
                let mut api = api_lock.lock().await;
                match api.refresh(&refresh_token, &client_id).await {
                    Err(e) => log::warn!("Couldn't refresh token: {e}"),
                    Ok(r) => {
                        refresh_token = r;
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn fetch_track<T: AsRef<str>>(&self, track: T, num: Option<u16>) -> Result<PathBuf> {
        let api = self.api.lock().await;
        let track = api
            .download_track(&track, num)
            .await
            .with_context(|| format!("Failed to download track: {}", track.as_ref()))?;

        self.fs
            .add_music(&track)
            .await
            .with_context(|| format!("Failed to save track to: {}", &track.display()))?;

        Ok(track)
    }

    pub async fn fetch_album<T: AsRef<str>>(&self, album: T) -> Result<()> {
        let api = self.api.lock().await;
        let tracks = api
            .get_tracks(album.as_ref())
            .await
            .with_context(|| format!("Failed to find tracks in album: {}", album.as_ref()))?;

        log::debug!("found tracks: {:?}", tracks);

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
