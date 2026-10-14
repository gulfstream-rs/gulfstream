use std::{
    pin::Pin,
    task::{Context, Poll},
};

use axum::body::Bytes;
use futures_util::Stream;
use tokio_util::io::ReaderStream;

use crate::{application::analytics, state::AppState};

pub(super) struct AnalyticsStream {
    inner: ReaderStream<tokio::io::Take<tokio::fs::File>>,
    state: AppState,
    account_id: String,
    media_id: String,
    session_id: Option<String>,
    bytes_emitted: u64,
    recording_scheduled: bool,
}
impl AnalyticsStream {
    pub(super) fn new(
        inner: ReaderStream<tokio::io::Take<tokio::fs::File>>,
        state: AppState,
        account_id: String,
        media_id: String,
        session_id: Option<String>,
    ) -> Self {
        Self {
            inner,
            state,
            account_id,
            media_id,
            session_id,
            bytes_emitted: 0,
            recording_scheduled: false,
        }
    }

    fn schedule_recording(&mut self) {
        if self.recording_scheduled || self.bytes_emitted == 0 {
            return;
        }
        self.recording_scheduled = true;
        let state = self.state.clone();
        let account_id = self.account_id.clone();
        let media_id = self.media_id.clone();
        let session_id = self.session_id.clone();
        let bytes = self.bytes_emitted;
        tokio::spawn(async move {
            if let Err(error) = analytics::record_bytes_served(
                &state,
                &account_id,
                &media_id,
                session_id.as_deref(),
                bytes,
            )
            .await
            {
                tracing::error!(%error, %media_id, "failed to persist streamed byte analytics");
            }
        });
    }
}

impl Stream for AnalyticsStream {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(context) {
            Poll::Ready(Some(Ok(bytes))) => {
                self.bytes_emitted = self.bytes_emitted.saturating_add(bytes.len() as u64);
                Poll::Ready(Some(Ok(bytes)))
            }
            Poll::Ready(None) => {
                self.schedule_recording();
                Poll::Ready(None)
            }
            other => other,
        }
    }
}

impl Drop for AnalyticsStream {
    fn drop(&mut self) {
        self.schedule_recording();
    }
}
