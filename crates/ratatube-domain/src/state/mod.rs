//! Domain state: everything the daemon owns and reducers mutate.

pub mod channel;
mod domain_state;
mod navigation;
mod operations;
mod prompt;
mod timers;

pub use channel::{ChannelNavigationSnapshot, ChannelState};
pub use domain_state::DomainState;
pub use navigation::{Focus, HistoryViewMode, HomeSection, PlayingPane, View};
pub use operations::{DetailsStatus, ImportState, OperationStatus, PendingResume};
pub use prompt::PromptPurpose;
pub use timers::SleepTimer;
