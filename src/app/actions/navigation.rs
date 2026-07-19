//! Navigation, search, and selection actions.

use crate::app::state::View;
use crate::media::Track;

/// An intent that changes the active view, search, or list selection.
#[derive(Debug, Clone)]
pub enum NavigationAction {
    Navigate(View),
    /// Open help while remembering the current view.
    OpenHelp,
    /// Return from help to the view that opened it.
    CloseHelp,
    /// Scroll the help document by signed rows.
    ScrollHelp(i32),
    NextView,
    PreviousView,
    /// Move Home section focus forward/backward.
    CycleHomeSection(i32),
    Quit,
    SearchInput(char),
    SearchBackspace,
    SubmitSearch(String),
    /// Fetch one exact video URL without routing it through search.
    SubmitExactVideo(String),
    SearchCompleted {
        generation: u64,
        tracks: Vec<Track>,
    },
    SearchFailed {
        generation: u64,
        message: String,
    },
    ClearSearch,
    /// Toggle the selected-result detail overlay on narrow Search layouts.
    ToggleSearchDetail,
    /// Open the selected Search result or current Playing track in a browser.
    OpenInBrowser,
    SelectNext,
    SelectPrevious,
}
