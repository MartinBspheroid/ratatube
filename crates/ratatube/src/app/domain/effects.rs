//! Execution of reducer-produced side effects.

use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::Action;
use crate::app::reducer::Effect;
use crate::history::model::PlaybackOutcome;

impl App {
    /// Execute side effects returned by the reducer.
    pub(in crate::app) async fn execute(
        &mut self,
        effects: Vec<Effect>,
        action_tx: &mpsc::Sender<Action>,
    ) {
        for effect in effects {
            match effect {
                Effect::RunSearch { query, generation } => {
                    self.spawn_search(query, generation, action_tx.clone());
                }
                Effect::RunExactVideo { url, generation } => {
                    self.spawn_exact_video(url, generation, action_tx.clone());
                }
                Effect::RunImport { url } => self.spawn_import(url, action_tx.clone()),
                Effect::ResolveAndPlay {
                    track_index_in_queue,
                } => {
                    self.state.domain.pending_resume = None;
                    self.spawn_resolve_and_play(track_index_in_queue, action_tx);
                }
                Effect::SeekBy(seconds) => {
                    if let Some(playback) = self.playback.as_mut() {
                        let _ = playback.queue_seek_by(seconds);
                    }
                }
                Effect::SeekTo(seconds) => {
                    if let Some(playback) = self.playback.as_mut() {
                        let _ = playback.queue_seek_to(seconds);
                    }
                }
                Effect::TogglePause => {
                    if let Some(playback) = self.playback.as_mut() {
                        let _ = playback.queue_toggle_pause();
                    }
                }
                Effect::AdjustVolume(delta) => {
                    if let Some(playback) = self.playback.as_mut() {
                        let _ = playback.queue_adjust_volume(delta);
                    }
                }
                Effect::ToggleMute => {
                    if let Some(playback) = self.playback.as_mut() {
                        let _ = playback.queue_toggle_mute();
                    }
                }
                Effect::SetSpeed(speed) => {
                    if let Some(playback) = self.playback.as_mut() {
                        let _ = playback.queue_set_speed(speed);
                    }
                }
                Effect::StopPlayback => {
                    self.record_current(PlaybackOutcome::Stopped);
                    if let Some(playback) = self.playback.as_mut() {
                        let _ = playback.queue_stop();
                    }
                }
                Effect::QuitMpv => {
                    if let Some(playback) = self.playback.as_mut() {
                        // The event loop is already exiting; wait for the final
                        // acknowledgement so dropping the controller cannot abort
                        // graceful mpv shutdown.
                        let _ = playback.quit().await;
                    }
                }
                Effect::PersistQueue => self.persist_queue(),
                Effect::PersistSession => {
                    self.maybe_save_session(self.state.domain.playback.position_seconds, true);
                }
                Effect::PersistUiSettings {
                    theme,
                    icons,
                    resume,
                } => {
                    self.config.ui.theme = theme;
                    self.config.ui.icons = icons;
                    self.config.playback.resume_on_launch = resume;
                    self.state.ui.icon_mode = crate::ui::icons::resolve_icon_mode(icons);
                    match crate::config::loader::save(&self.paths.config_file(), &self.config) {
                        Ok(()) => self.state.notify("Settings saved", false),
                        Err(err) => {
                            tracing::warn!(?err, "settings save failed");
                            self.state.notify("Settings could not be saved", true);
                        }
                    }
                }
                Effect::PersistPlaylists | Effect::Exit => {}
            }
        }
    }
}
