//! Operating-system adapters with validated, bounded process boundaries.

pub mod child;
pub mod clipboard;

/// Maximum accepted byte length for a URL passed to an operating-system command.
pub const MAX_EXTERNAL_URL_BYTES: usize = 2_048;

/// Validate one credential-free HTTPS YouTube video URL for process dispatch.
pub fn validate_youtube_video_url(url: &str) -> ratatube_domain::error::Result<()> {
    if url.len() > MAX_EXTERNAL_URL_BYTES {
        return Err(ratatube_domain::error::AppError::ResourceLimit {
            resource: "external URL".to_string(),
            limit: MAX_EXTERNAL_URL_BYTES,
        });
    }
    if is_safe_youtube_video_url(url) {
        Ok(())
    } else {
        Err(ratatube_domain::error::AppError::InvalidUrl(
            "expected an HTTPS YouTube video URL".to_string(),
        ))
    }
}

/// Return whether `url` is a credential-free HTTPS YouTube video URL.
pub fn is_safe_youtube_video_url(url: &str) -> bool {
    if url.len() > MAX_EXTERNAL_URL_BYTES {
        return false;
    }
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
