//! Identity for supervised asynchronous operations.
//!
//! The registry that owns cancellation and join handles is a service; only
//! the identity it stamps onto completion commands is domain vocabulary.

/// Monotonic identity attached to an asynchronous operation result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationId(u64);

impl OperationId {
    /// Mint the next identity; only the operation registry calls this.
    pub fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Placeholder identity for client-side mirrors of daemon-owned
    /// operations; the mirror never completes or cancels operations.
    pub fn mirror_placeholder() -> Self {
        Self(0)
    }
}

/// Independently supersedable operation families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationKind {
    Playback,
    Import,
    Radio,
    Details,
    Thumbnail,
    SearchThumbnail,
    Search,
    Prefetch,
    Mix,
    Session,
    PlaybackRecovery,
    ExternalCommand,
    ChannelResolve,
    ChannelPage,
}
