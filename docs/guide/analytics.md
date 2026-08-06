# Analytics

Analytics are derived from persisted playback activity, not generated sample data.

## Metrics

- **Views**: first accepted `play` event for a playback session.
- **Unique viewers**: exact per-account, per-media, per-day deduplication using a keyed visitor hash.
- **Play starts**: first play transition.
- **Completed views**: first completion transition.
- **Watch time**: accepted heartbeat deltas, bounded by the configured maximum.
- **Bytes served**: bytes actually emitted by source, playlist, and segment response streams.

The player creates a playback session when the watch page is opened, but that alone does not count as a view. Events are serialized per session so duplicate play/completion transitions are not double-counted.

## Queries

Both analytics endpoints accept optional `from`, `to`, and `media_id` parameters. Reporting length is bounded by configuration. Daily time series include zero-value dates for convenient charting.

## Retention

Raw events and old playback sessions have independent retention periods. Daily aggregates and exact daily unique counts remain available after raw-data cleanup. Set a retention period to zero to retain that class indefinitely.
