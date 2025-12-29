use base64::prelude::*;
use metaflac::Tag;
use reqwest::{self, header::HeaderValue};
use serde::Deserialize;
use std::path::Path;
use thiserror::Error;
use tokio::io::AsyncWriteExt;

#[derive(Error, Debug)]
pub enum AccessError {
    #[error("A problem occurred while sending an HTTP request: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Response wasn't typed as expected: {0}")]
    DeserializationFailure(#[from] serde_json::Error),
    #[error("Expected a manifest in the following response: {0}")]
    ManifestExpected(serde_json::Value),
    #[error("Couldn't decode manifest's Base64: {0}")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("Couldn't tag file: {0}")]
    AudioTagging(#[from] metaflac::Error),
}

type Result<T> = std::result::Result<T, AccessError>;

/// Handle auth for the API.
#[derive(Debug)]
pub struct Access {
    api_creds: ClientCredentials,
    streaming_creds: ClientCredentials,
    client: reqwest::Client,
}

/// Store the response of the /ouath2/token route.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ClientCredentials {
    access_token: String,
    /// Number of seconds before expiry.
    token_type: String,
    expires_in: u32,
}

#[derive(Debug)]
struct RelevantMetadata {
    title: String,
    artists: Vec<String>,
    album: String,
    track_number: Option<u16>,
}

impl RelevantMetadata {
    pub fn tag<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut tag = Tag::read_from_path(path)?;
        tag.set_vorbis("title", vec![&self.title]);

        let artist_tag = self.artists.join("; ");
        tag.set_vorbis("artist", vec![artist_tag]);
        tag.set_vorbis("album", vec![&self.album]);
        if let Some(tn) = self.track_number {
            tag.set_vorbis("track", vec![format!("{tn}")]);
        }

        tag.save()?;

        Ok(())
    }
}

impl Access {
    pub async fn log_in(client_id: &str, client_secret: &str, streaming_tok: &str) -> Result<Self> {
        let client = reqwest::Client::new();
        let mut auth_header: HeaderValue = format!(
            "Basic {}",
            BASE64_STANDARD.encode(format!("{client_id}:{client_secret}"))
        )
        .try_into()
        .expect("valid header value");

        auth_header.set_sensitive(true);

        let api_creds_raw = client
            .post("https://auth.tidal.com/v1/oauth2/token")
            .header("Authorization", auth_header)
            .body("grant_type=client_credentials")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()
            .await?
            .text()
            .await?;

        let api_creds: ClientCredentials = serde_json::from_str(&api_creds_raw)?;

        Ok(Self {
            client,
            api_creds,
            streaming_creds: ClientCredentials {
                access_token: streaming_tok.to_string(),
                token_type: "Bearer".to_string(),
                expires_in: 14400, // default; just a guess
            },
        })
    }

    pub async fn download_track(&self, track_id: &str) -> Result<()> {
        let metadata = self.get_metadata(track_id).await?;
        let manifest = self.get_manifest(track_id).await?;

        let o_path = format!("/tmp/{track_id}.flac");
        let mut child = tokio::process::Command::new("ffmpeg")
            .arg("-protocol_whitelist")
            .arg("fd,file,https,tcp,tls")
            .arg("-i")
            .arg("-")
            .arg("-c")
            .arg("copy")
            .arg(&o_path)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .expect("lol");

        {
            let mut child_stdin = child.stdin.take().unwrap();
            child_stdin.write_all(&manifest).await.unwrap();
            // put it in an inside scope to close the file
        }
        child.wait().await.unwrap();
        metadata.tag(o_path)?;

        Ok(())
    }

    async fn get_metadata(&self, track_id: &str) -> Result<RelevantMetadata> {
        let md = self
            .client
            .get(format!("https://openapi.tidal.com/v2/tracks/{track_id}"))
            .bearer_auth(&self.api_creds.access_token)
            .query(&[
                ("countryCode", "US"),
                ("include", "artists"),
                ("include", "albums"),
            ])
            .send()
            .await?
            .text()
            .await?;

        let serde_json::Value::Object(md) = serde_json::from_str(&md)? else {
            todo!()
        };

        let md = &md["data"];

        let serde_json::Value::String(title) = md["attributes"]["title"].clone() else {
            panic!("oops lmfao");
        };

        let serde_json::Value::String(album_id) =
            md["relationships"]["albums"]["data"][0]["id"].clone()
        else {
            panic!("sigh");
        };

        let album_md = self
            .client
            .get(format!("https://openapi.tidal.com/v2/albums/{album_id}"))
            .bearer_auth(&self.api_creds.access_token)
            .query(&[("countryCode", "US")])
            .send()
            .await?
            .text()
            .await?;

        let serde_json::Value::Object(album_md) = serde_json::from_str(&album_md)? else {
            todo!(":(")
        };

        let serde_json::Value::String(album) = album_md["data"]["attributes"]["title"].clone()
        else {
            panic!("af;jslkd");
        };

        let serde_json::Value::Array(artists) = &md["relationships"]["artists"]["data"] else {
            panic!("awawawwa");
        };

        let mut artist_names = Vec::new();

        for artist in artists {
            let serde_json::Value::Object(artist) = artist else {
                panic!("kys?");
            };
            let serde_json::Value::String(id) = &artist["id"] else {
                panic!("awf;elkj");
            };
            let artist_md = self
                .client
                .get(format!("https://openapi.tidal.com/v2/artists/{id}"))
                .bearer_auth(&self.api_creds.access_token)
                .query(&[("countryCode", "US")])
                .send()
                .await?
                .text()
                .await?;

            let serde_json::Value::Object(artist_md) = serde_json::from_str(&artist_md)? else {
                panic!("aweji");
            };

            let serde_json::Value::String(artist_name) =
                artist_md["data"]["attributes"]["name"].clone()
            else {
                panic!("meow");
            };

            artist_names.push(artist_name);
        }

        Ok(RelevantMetadata {
            title,
            album,
            artists: artist_names,
            track_number: None,
        })
    }

    async fn get_manifest(&self, track_id: &str) -> Result<Vec<u8>> {
        let playback_info = self.client
            .get(format!("https://tidal.com/v1/tracks/{track_id}/playbackinfo?audioquality=HI_RES_LOSSLESS&playbackmode=STREAM&assetpresentation=FULL"))
            .header("authorization", format!("Bearer {}", &self.streaming_creds.access_token))
            .header("User-Agent", "i just wanna download music please don't block me@tali.network 🥺") // we get blocked if we don't set the UA :(
            .send()
            .await?
            .text()
            .await?;

        let playback_info: serde_json::Value = serde_json::from_str(&playback_info)?;
        let serde_json::Value::String(manifest) = &playback_info["manifest"] else {
            return Err(AccessError::ManifestExpected(playback_info));
        };

        Ok(BASE64_STANDARD.decode(manifest)?)
    }
}
