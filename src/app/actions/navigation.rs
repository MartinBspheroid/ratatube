//! Navigation, search, and selection actions.

use crate::app::operations::OperationId;
use crate::app::state::View;
use crate::media::Track;

/// Operating-system command launched outside the application event loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalCommandKind {
    Browser,
    Clipboard,
}

/// UI surface that receives an external-command completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalCommandTarget {
    Direct,
    TrackContext { track_id: String },
}

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
    /// Complete one bounded browser or clipboard operation.
    ExternalCommandCompleted {
        operation_id: OperationId,
        command: ExternalCommandKind,
        target: ExternalCommandTarget,
        result: std::result::Result<(), String>,
    },
    /// Resolve the selected track and open its universal context menu.
    OpenTrackContext,
    /// Close the universal track context menu without executing an action.
    CloseTrackContext,
    /// Move the context-menu selection by a signed number of rows.
    MoveTrackContext(i32),
    /// Submit the selected menu action; Task 3 owns execution.
    SubmitTrackContext,
    /// Carry the selected track to Task 5 channel metadata resolution.
    VisitChannel(Track),
    /// Show details for the selected track without changing playback ownership.
    ShowTrackDetails(Track),
    /// Close the selected-track details modal.
    CloseTrackDetails,
    SelectNext,
    SelectPrevious,
}
