//! URL classification for pasted input (PRD section 10.2).

/// Classification of a supported URL or free-text query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputKind {
    /// Single YouTube video; carries the video ID.
    Video(String),
    /// YouTube or YouTube Music playlist; carries the playlist ID.
    Playlist(String),
    /// YouTube mix / radio URL; carries the video ID it starts from.
    Mix(String),
    /// Free-text search query.
    Query(String),
}

/// Classify raw user input into a URL kind or a plain search query.
pub fn classify_input(raw: &str) -> InputKind {
    let input = raw.trim();
    if let Some(id) = playlist_id(input) {
        // "RD"-prefixed lists are auto-generated mixes/radios (PRD 10.2).
        if id.starts_with("RD")
            && let Some(video) = video_id(input)
        {
            return InputKind::Mix(video.to_string());
        }
        return InputKind::Playlist(id.to_string());
    }
    if let Some(id) = video_id(input) {
        return InputKind::Video(id.to_string());
    }
    InputKind::Query(input.to_string())
}

/// Extract a YouTube video ID from watch, youtu.be, shorts, or
/// music.youtube.com watch URLs. Returns `None` for non-URL input.
fn video_id(input: &str) -> Option<&str> {
    if !looks_like_url(input) {
        return None;
    }
    if let Some(rest) = input.strip_prefix("https://youtu.be/") {
        return rest.split(['?', '&', '/']).next().filter(|s| !s.is_empty());
    }
    query_param(input, "v").or_else(|| {
        input
            .split("/shorts/")
            .nth(1)
            .and_then(|rest| rest.split(['?', '&', '/']).next())
            .filter(|s| !s.is_empty())
    })
}

/// Extract a playlist ID from a `list=` query parameter on any YouTube URL.
fn playlist_id(input: &str) -> Option<&str> {
    if !looks_like_url(input) {
        return None;
    }
    query_param(input, "list")
}

fn looks_like_url(input: &str) -> bool {
    input.starts_with("https://") || input.starts_with("http://") || input.contains("youtu")
}

fn query_param<'a>(input: &'a str, key: &str) -> Option<&'a str> {
    let query = input.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut kv = pair.splitn(2, '=');
        if kv.next() == Some(key)
            && let Some(value) = kv.next()
            && !value.is_empty()
        {
            return Some(value);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_watch_url() {
        assert_eq!(
            classify_input("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            InputKind::Video("dQw4w9WgXcQ".to_string())
        );
    }

    #[test]
    fn classifies_short_url() {
        assert_eq!(
            classify_input("https://youtu.be/dQw4w9WgXcQ?t=42"),
            InputKind::Video("dQw4w9WgXcQ".to_string())
        );
    }

    #[test]
    fn classifies_music_track_url() {
        assert_eq!(
            classify_input("https://music.youtube.com/watch?v=dQw4w9WgXcQ"),
            InputKind::Video("dQw4w9WgXcQ".to_string())
        );
    }

    #[test]
    fn classifies_playlist_url() {
        assert_eq!(
            classify_input("https://www.youtube.com/playlist?list=PLabc123"),
            InputKind::Playlist("PLabc123".to_string())
        );
        assert_eq!(
            classify_input("https://music.youtube.com/playlist?list=OLAK5uy_x"),
            InputKind::Playlist("OLAK5uy_x".to_string())
        );
    }

    #[test]
    fn classifies_mix_url() {
        assert_eq!(
            classify_input("https://www.youtube.com/watch?v=abc&list=RDabc"),
            InputKind::Mix("abc".to_string())
        );
    }

    #[test]
    fn free_text_is_query() {
        assert_eq!(
            classify_input("boards of canada"),
            InputKind::Query("boards of canada".to_string())
        );
    }

    #[test]
    fn watch_with_regular_playlist_is_playlist_first() {
        // A watch URL that also carries a normal list= belongs to a playlist.
        assert_eq!(
            classify_input("https://www.youtube.com/watch?v=abc&list=PLxyz"),
            InputKind::Playlist("PLxyz".to_string())
        );
    }
}
