# Operations

## Health checks

- `/health/live` confirms the process is serving requests.
- `/health/ready` verifies SQLite, storage directories, and configured FFmpeg/FFprobe executables.

## Backups

Back up the SQLite database and storage tree as one consistency unit. Pause writes or use an SQLite-safe snapshot procedure. Test restore procedures regularly.

## Scaling

Multiple worker tasks are supported in one process. Job leasing and publication fencing protect against duplicate ownership. SQLite is appropriate for a single-node deployment; moving to a multi-node control plane would require a database and storage implementation with equivalent transactional and atomic-publication guarantees.

## Logging

`observability.log_filter` accepts a tracing filter. Output can be human-readable or JSON. Request spans log methods and paths but deliberately omit query strings so secrets cannot leak through URLs.

## Storage lifecycle

`storage.preserve_deleted_media` selects permanent removal or movement to the configured trash directory. `transcoding.retain_original` controls whether the source remains after successful publication.
