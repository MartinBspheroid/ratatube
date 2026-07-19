//! Versioned JSON playlist parsing and validation.

use serde::Deserialize;

use crate::media::Track;
use crate::playlists::import::deduplicate;
use crate::playlists::model::{Playlist, PlaylistTrack};

const VERSION: u32 = 1;
const MAX_BYTES: usize = 1_048_576;
const MAX_PLAYLISTS: usize = 50;
const MAX_TRACKS: usize = 10_000;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: u32,
    playlists: Vec<JsonPlaylist>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonPlaylist {
    name: String,
    #[serde(default)]
    description: String,
    tracks: Vec<JsonTrack>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonTrack {
    title: String,
    channel: String,
    url: String,
}

pub(super) fn parse(input: &str) -> Result<Vec<Playlist>, String> {
    let document = parse_document(input)?;
    document.playlists.into_iter().map(build_playlist).collect()
}

fn parse_document(input: &str) -> Result<Document, String> {
    if input.len() > MAX_BYTES {
        return Err("JSON import exceeds the 1 MiB limit".to_string());
    }
    let document: Document =
        serde_json::from_str(input).map_err(|error| format!("Invalid JSON: {error}"))?;
    if document.version != VERSION {
        return Err(format!(
            "Unsupported JSON import version {}; expected {VERSION}",
            document.version
        ));
    }
    if document.playlists.is_empty() {
        return Err("JSON import must contain at least one playlist".to_string());
    }
    if document.playlists.len() > MAX_PLAYLISTS {
        return Err(format!(
            "JSON import supports at most {MAX_PLAYLISTS} playlists"
        ));
    }
    let track_count = document
        .playlists
        .iter()
        .map(|playlist| playlist.tracks.len())
        .sum::<usize>();
    if track_count > MAX_TRACKS {
        return Err(format!("JSON import supports at most {MAX_TRACKS} tracks"));
    }
    Ok(document)
}

fn build_playlist(source: JsonPlaylist) -> Result<Playlist, String> {
    let name = source.name.trim();
    if name.is_empty() {
        return Err("Every imported playlist needs a non-empty name".to_string());
    }
    if source.tracks.is_empty() {
        return Err(format!("Playlist \"{name}\" contains no tracks"));
    }
    let tracks = source
        .tracks
        .into_iter()
        .map(|track| build_track(name, track))
        .collect::<Result<Vec<_>, _>>()?;
    let (tracks, _) = deduplicate(tracks);
    let mut playlist = Playlist::new(name);
    playlist.description = source.description.trim().to_string();
    playlist.tracks = tracks.iter().map(PlaylistTrack::from).collect();
    Ok(playlist)
}

fn build_track(playlist_name: &str, item: JsonTrack) -> Result<Track, String> {
    let title = item.title.trim();
    let channel = item.channel.trim();
    let url = item.url.trim();
    let id = match crate::media::import::classify_input(url) {
        crate::media::import::InputKind::Video(id) => id,
        _ => {
            return Err(format!(
                "Playlist \"{playlist_name}\", track \"{title}\" needs a YouTube video URL"
            ));
        }
    };
    if title.is_empty() || channel.is_empty() {
        return Err(format!(
            "Playlist \"{playlist_name}\" has a track with an empty title or channel"
        ));
    }
    let mut track = Track::new(id, title, channel);
    track.webpage_url = url.to_string();
    Ok(track)
}
