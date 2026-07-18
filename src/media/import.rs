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
    let (is_short_host, path_and_query) = youtube_url(input)?;
    if is_short_host {
        return path_and_query
            .strip_prefix('/')?
            .split(['?', '&', '/', '#'])
            .next()
            .filter(|id| valid_id(id));
    }
    query_param(path_and_query, "v").or_else(|| {
        path_and_query
            .split("/shorts/")
            .nth(1)
            .and_then(|rest| rest.split(['?', '&', '/', '#']).next())
            .filter(|id| valid_id(id))
    })
}

/// Extract a playlist ID from a `list=` query parameter on any YouTube URL.
fn playlist_id(input: &str) -> Option<&str> {
    let (_, path_and_query) = youtube_url(input)?;
    query_param(path_and_query, "list").filter(|id| valid_id(id))
}

fn youtube_url(input: &str) -> Option<(bool, &str)> {
    let (scheme, remainder) = input.split_once("://")?;
    if !matches!(scheme, "http" | "https") {
        return None;
    }
    let authority_end = remainder.find(['/', '?', '#']).unwrap_or(remainder.len());
    let authority = &remainder[..authority_end];
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    let host = authority.split(':').next()?.to_ascii_lowercase();
    let allowed = matches!(
        host.as_str(),
        "youtube.com" | "www.youtube.com" | "m.youtube.com" | "music.youtube.com" | "youtu.be"
    );
    allowed.then_some((host == "youtu.be", &remainder[authority_end..]))
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

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
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

    #[test]
    fn rejects_lookalike_hosts_and_non_url_youtube_text() {
        for input in [
            "https://youtube.com.evil.example/watch?v=attack",
            "https://evil-youtube.com/watch?v=attack",
            "https://youtube.com@evil.example/watch?v=attack",
            "javascript://youtube.com/watch?v=attack",
            "please search youtu.be/attack",
        ] {
            assert_eq!(
                classify_input(input),
                InputKind::Query(input.to_string()),
                "input: {input}"
            );
        }
    }
}
