//! In-list filtering: the `/` filter available in Queue, Playlists,
//! Playlist detail, and History.
//!
//! Plain tokens match case-insensitively against the row's text; tokens
//! starting with `>` match the playback outcome in History (e.g.
//! `>completed`, `>skip`).

/// Whether a row matches the filter string.
///
/// Every whitespace-separated token must match: `>x` tokens against
/// `outcome` (prefix match), plain tokens as substrings of `text`.
pub fn matches(filter: &str, text: &str, outcome: Option<&str>) -> bool {
    let text = text.to_lowercase();
    filter.split_whitespace().all(|token| {
        if let Some(wanted) = token.strip_prefix('>') {
            outcome.is_some_and(|o| o.to_lowercase().starts_with(&wanted.to_lowercase()))
        } else {
            text.contains(&token.to_lowercase())
        }
    })
}

/// Indices of `rows` that match `filter`; each row is (text, outcome).
pub fn matching_indices<'a>(
    filter: &str,
    rows: impl Iterator<Item = (String, Option<&'a str>)>,
) -> Vec<usize> {
    rows.enumerate()
        .filter(|(_, (text, outcome))| matches(filter, text, *outcome))
        .map(|(i, _)| i)
        .collect()
}

/// Candidates for the add-to-playlist picker under the current filter:
/// whether a "create new playlist" entry leads the list, plus the indices
/// of matching playlists.
pub fn picker_candidates(
    playlists: &[ratatube_domain::playlists::Playlist],
    filter: &str,
) -> (bool, Vec<usize>) {
    let matching: Vec<usize> = playlists
        .iter()
        .enumerate()
        .filter(|(_, p)| matches(filter, &p.name, None))
        .map(|(i, _)| i)
        .collect();
    let trimmed = filter.trim();
    let exact = playlists
        .iter()
        .any(|p| p.name.eq_ignore_ascii_case(trimmed));
    (!trimmed.is_empty() && !exact, matching)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_offers_create_for_novel_names() {
        let playlists = vec![
            ratatube_domain::playlists::Playlist::new("Techno Sets"),
            ratatube_domain::playlists::Playlist::new("Ambient"),
        ];
        let (create, matching) = picker_candidates(&playlists, "tech");
        assert!(create);
        assert_eq!(matching, vec![0]);
        // Exact name match: no create entry.
        let (create, _) = picker_candidates(&playlists, "ambient");
        assert!(!create);
        // Empty filter: all playlists, no create entry.
        let (create, matching) = picker_candidates(&playlists, "");
        assert!(!create);
        assert_eq!(matching.len(), 2);
    }

    #[test]
    fn plain_tokens_match_substrings_case_insensitively() {
        assert!(matches("skee mask", "Skee Mask — Essential Mix", None));
        assert!(!matches("aphex", "Skee Mask — Essential Mix", None));
        assert!(matches("", "anything", None));
    }

    #[test]
    fn outcome_tokens_match_outcome_prefix() {
        assert!(matches(">comp", "whatever", Some("Completed")));
        assert!(!matches(">skip", "whatever", Some("Completed")));
        assert!(matches("mask >comp", "Skee Mask", Some("Completed")));
        assert!(!matches(">comp", "no outcome here", None));
    }

    #[test]
    fn indices_are_filtered() {
        let rows = vec![
            ("Skee Mask — ISS006".to_string(), None),
            ("Aphex Twin — Xtal".to_string(), None),
            ("Skee Mask — Rev8617".to_string(), None),
        ];
        let indices = matching_indices("skee", rows.into_iter());
        assert_eq!(indices, vec![0, 2]);
    }
}
