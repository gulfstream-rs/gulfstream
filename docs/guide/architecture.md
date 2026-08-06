# Architecture

Gulfstream uses explicit boundaries and a deliberately small public Rust API:

- **Domain** defines account, media, rendition, job, visibility, status, and analytics values.
- **Application** implements account, session, upload, media, processing, dashboard, and analytics use cases.
- **Infrastructure** owns SQLite connections, filesystem layout, FFprobe inspection, and FFmpeg execution.
- **HTTP** translates configured routes and authentication into application calls and response contracts.
- **Workers** lease durable jobs, renew leases, retry failures, fence publication, and perform retention cleanup.
- **Web presentation** provides dependency-free HTML, CSS, and ES modules that consume the public API.
- **Runtime** initializes tracing, state, routes, workers, graceful shutdown, and the network listener.

External Rust users see only the configuration types, configured-path helper, and runtime entry point. Internal modules remain private so implementation changes do not accidentally become a public compatibility commitment.

## Data flow

1. A direct upload is streamed to temporary storage, bounded, hashed, and moved to durable account storage.
2. A remote import stores a durable job first. The worker validates DNS and every redirect, streams the source to bounded storage, and records its checksum.
3. A transcode job probes the source and selects configured profiles that do not violate the upscale policy.
4. FFmpeg writes each rendition into a staging directory. Gulfstream validates outputs and atomically publishes the stream directory and database rows.
5. Playback creates a persisted session. Player events update daily aggregates and exact daily unique-viewer records.
6. Stream wrappers record bytes actually emitted, including partial and abandoned responses.

## Durability and concurrency

Jobs are stored in SQLite, claimed with leases, renewed while external processes run, and fenced during final publication. Retries use configured attempts and backoff. Deletion cancels queued work, waits on active media work, and prevents stale workers from republishing deleted content.

Pagination and status parsing are centralized at the HTTP/domain boundaries. Application pages use a shared response type, avoiding duplicated response contracts across media and processing endpoints.
