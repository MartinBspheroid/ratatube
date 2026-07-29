mod channel;
/// Shared with the crate's unit tests, which cannot see an integration-test
/// module — see the file's own header.
#[path = "../support/fake_executable.rs"]
mod fake_executable;
mod live;
mod mocked;
mod support;
