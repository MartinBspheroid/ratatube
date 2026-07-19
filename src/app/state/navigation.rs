//! Navigation and focus state.

/// Primary views (PRD section 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    /// Landing dashboard: resume card, recent tracks, playlists.
    #[default]
    Home,
    Search,
    Queue,
    Playlists,
    PlaylistDetail,
    History,
    NowPlaying,
    Help,
}

impl View {
    /// Display label for the header.
    pub fn title(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Search => "Search",
            Self::Queue => "Queue",
            Self::Playlists => "Playlists",
            Self::PlaylistDetail => "Playlist",
            Self::History => "History",
            Self::NowPlaying => "Now Playing",
            Self::Help => "Help",
        }
    }

    /// Tab order shown in the header (excludes detail/help views).
    pub const TABS: [Self; 6] = [
        Self::Home,
        Self::Search,
        Self::Queue,
        Self::Playlists,
        Self::History,
        Self::NowPlaying,
    ];

    /// Next view in tab order, wrapping around.
    pub fn next_tab(self) -> Self {
        let position = Self::TABS
            .iter()
            .position(|view| *view == self)
            .unwrap_or(0);
        Self::TABS[(position + 1) % Self::TABS.len()]
    }

    /// Previous view in tab order, wrapping around.
    pub fn prev_tab(self) -> Self {
        let position = Self::TABS
            .iter()
            .position(|view| *view == self)
            .unwrap_or(0);
        Self::TABS[(position + Self::TABS.len() - 1) % Self::TABS.len()]
    }
}

/// Focusable sections of the Home dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HomeSection {
    #[default]
    Resume,
    Recent,
    Playlists,
}

impl HomeSection {
    /// Cycle focus forward (+1) or backward (-1).
    pub fn cycled(self, delta: i32) -> Self {
        const ORDER: [HomeSection; 3] = [
            HomeSection::Resume,
            HomeSection::Recent,
            HomeSection::Playlists,
        ];
        let position = ORDER
            .iter()
            .position(|section| *section == self)
            .unwrap_or(0) as i32;
        let next = (position + delta).rem_euclid(ORDER.len() as i32);
        ORDER[next as usize]
    }
}

/// Keyboard focus within the ultra-wide Playing view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayingPane {
    #[default]
    Info,
    Queue,
}

/// Which pane currently holds keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    /// The search input field is capturing text.
    SearchInput,
    /// The in-list filter bar is capturing text.
    ListFilter,
    /// The main content list.
    #[default]
    Content,
}

/// How the History view presents its entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistoryViewMode {
    /// Chronological log, newest first.
    #[default]
    Recent,
    /// Aggregated per track.
    Top,
}

#[cfg(test)]
mod tests {
    use super::HomeSection;

    #[test]
    fn home_section_ring_follows_visual_reading_order() {
        let mut section = HomeSection::Resume;
        let mut visited = Vec::new();
        for _ in 0..3 {
            visited.push(section);
            section = section.cycled(1);
        }
        assert_eq!(
            visited,
            vec![
                HomeSection::Resume,
                HomeSection::Recent,
                HomeSection::Playlists,
            ]
        );
        assert_eq!(section, HomeSection::Resume);
        assert_eq!(HomeSection::Resume.cycled(-1), HomeSection::Playlists);
    }
}
