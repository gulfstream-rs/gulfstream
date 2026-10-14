//! Public watch page, playback analytics events and media streaming routes.

mod assets;
mod oembed;
mod range;
mod stream;
mod watch;

use crate::{application::media::MediaRecord, error::AppError, state::AppState};

pub use assets::{master_playlist, source, variant_asset};
pub use oembed::oembed;
pub use watch::{embed, playback_event, watch};

fn ensure_ready(record: &MediaRecord) -> Result<(), AppError> {
    if record.status == "ready" {
        Ok(())
    } else {
        Err(AppError::MediaNotReady)
    }
}

/// Whether viewers may fetch the original upload. It is always available
/// when no HLS renditions exist, because it is then the only playable file.
fn original_exposed(state: &AppState, hls_available: bool) -> bool {
    !hls_available || state.config.protection.expose_original_with_hls
}
