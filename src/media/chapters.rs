use serde::{Deserialize, Serialize};

/// A chapter within a video: yt-dlp chapters when the uploader set them,
/// or timestamps parsed from a tracklist in the description (DJ mixes).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    pub title: String,
    pub start_seconds: f64,
}

/// Index of the chapter containing `position_seconds`, if any.
pub fn chapter_at(chapters: &[Chapter], position_seconds: f64) -> Option<usize> {
    chapters
        .iter()
        .rposition(|chapter| chapter.start_seconds <= position_seconds)
}

/// Parse a tracklist with timestamps out of a video description.
///
/// Accepts lines containing an `H:MM:SS` or `M:SS` token; the rest of the
/// line becomes the title. Only a list of two or more entries with
/// non-decreasing timestamps counts — anything else is noise, not a
/// tracklist.
pub fn parse_chapters_from_description(description: &str) -> Vec<Chapter> {
    fn parse_hms(token: &str) -> Option<f64> {
        let parts: Vec<&str> = token.split(':').collect();
        if !(2..=3).contains(&parts.len()) {
            return None;
        }
        let mut seconds = 0u64;
        for (index, part) in parts.iter().enumerate() {
            if part.is_empty()
                || part.len() > 2
                || !part.chars().all(|character| character.is_ascii_digit())
            {
                return None;
            }
            let value: u64 = part.parse().ok()?;
            if index > 0 && parts.len() == 3 && value >= 60 {
                return None;
            }
            if index + 1 == parts.len() && value >= 60 {
                return None;
            }
            seconds = seconds * 60 + value;
        }
        Some(seconds as f64)
    }

    let mut chapters = Vec::new();
    for line in description.lines() {
        let mut timestamp = None;
        let mut title_tokens = Vec::new();
        for token in line.split_whitespace() {
            let cleaned = token
                .trim_matches(|character: char| !character.is_ascii_digit() && character != ':');
            match parse_hms(cleaned) {
                Some(seconds) if timestamp.is_none() => timestamp = Some(seconds),
                Some(_) => {}
                None => title_tokens.push(token),
            }
        }
        if let Some(start_seconds) = timestamp {
            let title = title_tokens
                .join(" ")
                .trim_matches(|character: char| {
                    character.is_whitespace()
                        || matches!(
                            character,
                            '-' | '–' | '—' | '|' | ':' | '.' | '(' | ')' | '[' | ']' | '*'
                        )
                })
                .to_string();
            chapters.push(Chapter {
                title: if title.is_empty() {
                    format!("Chapter {}", chapters.len() + 1)
                } else {
                    title
                },
                start_seconds,
            });
        }
    }
    let increasing = chapters
        .windows(2)
        .all(|window| window[1].start_seconds >= window[0].start_seconds);
    if chapters.len() >= 2 && increasing {
        chapters
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Chapter, chapter_at, parse_chapters_from_description};

    #[test]
    fn parses_tracklist_with_timestamps() {
        let description = "Tracklist:\n0:00 Intro\n3:45 Artist - Song One\n1:02:11 Artist Two - Closer\nthanks for listening";
        let chapters = parse_chapters_from_description(description);
        assert_eq!(chapters.len(), 3);
        assert_eq!(chapters[0].title, "Intro");
        assert_eq!(chapters[1].start_seconds, 225.0);
        assert_eq!(chapters[2].start_seconds, 3731.0);
        assert_eq!(chapters[2].title, "Artist Two - Closer");
    }

    #[test]
    fn ignores_descriptions_without_a_tracklist() {
        assert!(parse_chapters_from_description("check my mix at 3:45 somewhere").is_empty());
        assert!(parse_chapters_from_description("no timestamps here at all").is_empty());
    }

    #[test]
    fn rejects_decreasing_timestamps() {
        assert!(parse_chapters_from_description("5:00 Later\n0:30 Earlier").is_empty());
    }

    #[test]
    fn chapter_at_finds_current() {
        let chapters = vec![
            Chapter {
                title: "a".into(),
                start_seconds: 0.0,
            },
            Chapter {
                title: "b".into(),
                start_seconds: 100.0,
            },
            Chapter {
                title: "c".into(),
                start_seconds: 200.0,
            },
        ];
        assert_eq!(chapter_at(&chapters, 0.0), Some(0));
        assert_eq!(chapter_at(&chapters, 150.0), Some(1));
        assert_eq!(chapter_at(&chapters, 999.0), Some(2));
        assert_eq!(chapter_at(&[], 10.0), None);
    }
}
