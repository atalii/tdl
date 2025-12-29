use metaflac::Tag;
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum FsError {
    #[error("Can't create directory: {0}: {1}")]
    CantCreate(PathBuf, std::io::Error),
    #[error("Can't copy track into store: {0}: {1}")]
    CantCopyTrack(PathBuf, std::io::Error),
    #[error("Can't read tags from file: {0}: {1}")]
    CantReadTags(PathBuf, metaflac::Error),
    #[error("Track is missing metadata for the album: {0}")]
    MissingAlbum(PathBuf),
    #[error("Track is missing metadata for the artist: {0}")]
    MissingArtist(PathBuf),
    #[error("Track is missing metadata for the title: {0}")]
    MissingTitle(PathBuf),
}

pub type Result<T> = std::result::Result<T, FsError>;

pub struct Dir {
    root: PathBuf,
}

impl Dir {
    /// Open up the directory.
    pub async fn new<P: AsRef<Path>>(root: P) -> Result<Self> {
        let root = root.as_ref().to_owned();
        tokio::fs::create_dir_all(&root)
            .await
            .map_err(|e| FsError::CantCreate(root.clone(), e))?;

        Ok(Self { root })
    }

    /// File away a piece of music into a tag-appropriate location.
    pub async fn add_music<P: AsRef<Path>>(&self, track: P) -> Result<()> {
        let tags = Tag::read_from_path(track.as_ref())
            .map_err(|e| FsError::CantReadTags(track.as_ref().to_owned(), e))?;

        let album = tags
            .get_vorbis("album")
            .ok_or(FsError::MissingAlbum(track.as_ref().to_owned()))?
            .next()
            .ok_or(FsError::MissingAlbum(track.as_ref().to_owned()))?;

        let artist = tags
            .get_vorbis("artist")
            .ok_or(FsError::MissingArtist(track.as_ref().to_owned()))?
            .next()
            .ok_or(FsError::MissingArtist(track.as_ref().to_owned()))?;

        let title = tags
            .get_vorbis("title")
            .ok_or(FsError::MissingTitle(track.as_ref().to_owned()))?
            .next()
            .ok_or(FsError::MissingTitle(track.as_ref().to_owned()))?;

        // XXX: This is a vulnerability if album, title, or artist contain slashes. It's also super
        // brittle if the title contains a period (and therefore a 'file extension').
        let dst_dir = self.root.join(artist).join(album);
        let dst = dst_dir.join(title).with_extension("flac");

        tokio::fs::create_dir_all(&dst_dir)
            .await
            .map_err(|e| FsError::CantCreate(dst_dir.clone(), e))?;

        // TODO: make sure we're not overwriting anything
        tokio::fs::copy(track, &dst)
            .await
            .map_err(|e| FsError::CantCopyTrack(dst.clone(), e))?;

        Ok(())
    }
}
