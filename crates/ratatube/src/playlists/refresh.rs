//! Imported playlist refresh diffing (PRD 10.9).
//!
//! Matching is by YouTube video ID. Local metadata is preserved where
//! possible; the default strategy previews changes before applying.

use crate::media::Track;
use crate::playlists::model::Playlist;

/// How a refresh should be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RefreshStrategy {
    /// Show the diff; apply nothing yet.
    #[default]
    Preview,
    /// Only append newly discovered tracks.
    AddNewOnly,
    /// Make the local playlist identical to the remote.
    Mirror,
}

/// Difference between a local playlist and freshly fetched remote tracks.
#[derive(Debug, Clone, Default)]
pub struct RefreshDiff {
    /// Present remotely but not locally.
    pub added: Vec<Track>,
    /// Present locally but no longer on the remote playlist.
    pub removed: Vec<String>,
    /// Present in both; local metadata is kept.
    pub kept: usize,
}

/// Compute the diff between local `playlist` and remote `tracks`.
pub fn diff(playlist: &Playlist, remote: &[Track]) -> RefreshDiff {
    let local_ids: std::collections::HashSet<&str> =
        playlist.tracks.iter().map(|t| t.id.as_str()).collect();
    let remote_ids: std::collections::HashSet<&str> =
        remote.iter().map(|t| t.id.as_str()).collect();

    let added = remote
        .iter()
        .filter(|t| !local_ids.contains(t.id.as_str()))
        .cloned()
        .collect();
    let removed = playlist
        .tracks
        .iter()
        .filter(|t| !remote_ids.contains(t.id.as_str()))
        .map(|t| t.id.clone())
        .collect();
    let kept = playlist
        .tracks
        .iter()
        .filter(|t| remote_ids.contains(t.id.as_str()))
        .count();

    RefreshDiff {
        added,
        removed,
        kept,
    }
}

/// Apply `diff_result` to `playlist` according to `strategy`.
/// Returns the number of removed tracks (for confirmation messaging).
pub fn apply(
    playlist: &mut Playlist,
    diff_result: RefreshDiff,
    strategy: RefreshStrategy,
) -> usize {
    let added: Vec<_> = diff_result
        .added
        .iter()
        .map(crate::playlists::model::PlaylistTrack::from)
        .collect();
    let removed_count = diff_result.removed.len();

    match strategy {
        RefreshStrategy::Preview => 0,
        RefreshStrategy::AddNewOnly => {
            playlist.tracks.extend(added);
            0
        }
        RefreshStrategy::Mirror => {
            playlist
                .tracks
                .retain(|t| !diff_result.removed.contains(&t.id));
            playlist.tracks.extend(added);
            removed_count
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playlists::model::PlaylistTrack;

    fn track(id: &str) -> Track {
        Track::new(id, id, "artist")
    }

    fn playlist_with(ids: &[&str]) -> Playlist {
        let mut p = Playlist::new("p");
        p.tracks = ids
            .iter()
            .map(|id| PlaylistTrack::from(&track(id)))
            .collect();
        p
    }

    #[test]
    fn diff_detects_added_removed_kept() {
        let playlist = playlist_with(&["a", "b", "c"]);
        let remote = vec![track("b"), track("c"), track("d")];
        let d = diff(&playlist, &remote);
        assert_eq!(d.added.len(), 1);
        assert_eq!(d.added[0].id, "d");
        assert_eq!(d.removed, vec!["a".to_string()]);
        assert_eq!(d.kept, 2);
    }

    #[test]
    fn preview_applies_nothing() {
        let mut playlist = playlist_with(&["a"]);
        let d = diff(&playlist, &[track("b")]);
        apply(&mut playlist, d, RefreshStrategy::Preview);
        assert_eq!(playlist.tracks.len(), 1);
    }

    #[test]
    fn mirror_adds_and_removes() {
        let mut playlist = playlist_with(&["a", "b"]);
        let d = diff(&playlist, &[track("b"), track("c")]);
        let removed = apply(&mut playlist, d, RefreshStrategy::Mirror);
        assert_eq!(removed, 1);
        let ids: Vec<&str> = playlist.tracks.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["b", "c"]);
    }
}
