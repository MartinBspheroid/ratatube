//! Operating-system adapters with validated, bounded process boundaries.

pub mod clipboard;

/// Return whether `url` is a credential-free HTTPS YouTube video URL.
pub(crate) fn is_safe_youtube_video_url(url: &str) -> bool {
    if url.chars().any(char::is_whitespace) || url.chars().any(char::is_control) {
        return false;
    }
    let Some(remainder) = url.strip_prefix("https://") else {
        return false;
    };
    let authority_end = remainder.find(['/', '?', '#']).unwrap_or(remainder.len());
    let authority = &remainder[..authority_end];
    if authority.is_empty() || authority.contains('@') || authority.contains(':') {
        return false;
    }
    let suffix = &remainder[authority_end..];
    match authority {
        "youtube.com" | "www.youtube.com" | "music.youtube.com" => {
            let Some(query) = suffix.strip_prefix("/watch?") else {
                return false;
            };
            query
                .split('&')
                .find_map(|pair| pair.strip_prefix("v="))
                .is_some_and(is_video_id)
        }
        "youtu.be" => suffix
            .strip_prefix('/')
            .and_then(|path| path.split(['?', '#']).next())
            .is_some_and(is_video_id),
        _ => false,
    }
}

fn is_video_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}
