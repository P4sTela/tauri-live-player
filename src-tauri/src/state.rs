use parking_lot::Mutex;

use crate::output::manager::OutputManager;
use crate::output::standby::StandbyManager;
use crate::pipeline::cue_player::CuePlayer;
use crate::types::Project;

pub struct AppState {
    pub player: Mutex<Option<CuePlayer>>,
    pub output_manager: Mutex<OutputManager>,
    pub standby_manager: Mutex<StandbyManager>,
    pub project: Mutex<Option<Project>>,
    pub current_cue_index: Mutex<i32>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            player: Mutex::new(None),
            output_manager: Mutex::new(OutputManager::new()),
            standby_manager: Mutex::new(StandbyManager::new()),
            project: Mutex::new(None),
            current_cue_index: Mutex::new(-1),
        }
    }

    pub fn init_player(&self) -> Result<(), gstreamer::glib::Error> {
        gstreamer::init()?;
        let player = CuePlayer::new()?;
        *self.player.lock() = Some(player);
        Ok(())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
