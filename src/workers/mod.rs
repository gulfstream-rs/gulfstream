use tokio::task::JoinHandle;

use crate::state::AppState;

pub mod maintenance;
pub mod media;

pub fn spawn(state: AppState) -> Vec<JoinHandle<()>> {
    let mut handles = media::spawn(state.clone());
    handles.push(maintenance::spawn(state));
    handles
}
