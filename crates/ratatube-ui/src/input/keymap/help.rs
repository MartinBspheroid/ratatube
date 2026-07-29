//! User-facing key catalog.

/// Canonical user-facing command catalog rendered by the Help view.
pub(crate) const HELP_SECTIONS: &[(&str, &[(&str, &str)])] = &[
    (
        "Views",
        &[
            ("1", "Home"),
            ("2", "Search"),
            ("3", "Queue"),
            ("4", "Playlists"),
            ("5", "History"),
            ("6", "Now Playing"),
        ],
    ),
    (
        "Playback",
        &[
            ("Space", "Play / pause (resume on Home)"),
            ("n / b", "Next / previous track"),
            (". / ,", "Next / previous chapter"),
            ("h / l", "Seek 5 seconds"),
            ("H / L", "Seek 30 seconds"),
            ("+ / -", "Volume"),
            ("m", "Mute"),
            ("s", "Shuffle"),
            ("r", "Repeat mode"),
            ("t", "Radio mode (auto-refill queue)"),
            ("< / >", "Playback speed (= resets)"),
            ("Z", "Sleep timer 15/30/60 min"),
        ],
    ),
    (
        "Lists",
        &[
            ("j / k", "Move selection"),
            ("Enter", "Play / open"),
            ("c", "Track actions"),
            ("a / A", "Add to queue / play next"),
            ("J / K", "Move item down / up"),
            ("d / u", "Remove queue item / undo removal"),
            ("C", "Clear queue (asks first)"),
            ("P", "Add to playlist..."),
            ("/", "Filter the list"),
        ],
    ),
    (
        "Playlists",
        &[
            ("Enter", "Open playlist editor"),
            ("p", "Play playlist"),
            ("i", "Import from URL"),
            ("I", "Import pasted JSON"),
            ("e", "Edit playlist name and description"),
            ("N", "New playlist"),
            ("R", "Rename"),
            ("x", "Delete (asks first)"),
            ("w", "Save queue as playlist"),
        ],
    ),
    (
        "History",
        &[
            ("g", "Toggle recent / top"),
            ("x", "Delete entry"),
            ("C", "Clear history (asks first)"),
        ],
    ),
    (
        "Other",
        &[
            ("/", "Search (outside lists)"),
            ("ctrl+p", "Settings (themes and options)"),
            ("!", "Message log"),
            ("v", "Chapters / description pane"),
            ("o", "Open selected/current track in browser"),
            ("?", "This help / return"),
            ("q", "Quit"),
        ],
    ),
];
