PRAGMA foreign_keys = ON;

CREATE TABLE accounts (
    id TEXT PRIMARY KEY NOT NULL,
    email TEXT NOT NULL UNIQUE COLLATE NOCASE,
    display_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'suspended')),
    storage_quota_bytes INTEGER NOT NULL CHECK (storage_quota_bytes > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE api_keys (
    id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    key_id TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    secret_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_used_at TEXT,
    revoked_at TEXT
);
CREATE INDEX api_keys_account_id_idx ON api_keys(account_id);

CREATE TABLE browser_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    csrf_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_used_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);
CREATE INDEX browser_sessions_account_idx ON browser_sessions(account_id, created_at DESC);
CREATE INDEX browser_sessions_expiry_idx ON browser_sessions(expires_at);

CREATE TABLE media_files (
    id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    source_filename TEXT NOT NULL,
    original_object_key TEXT,
    source_url TEXT,
    source_mime_type TEXT,
    source_size_bytes INTEGER NOT NULL DEFAULT 0 CHECK (source_size_bytes >= 0),
    storage_bytes INTEGER NOT NULL DEFAULT 0 CHECK (storage_bytes >= 0),
    sha256 TEXT,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    visibility TEXT NOT NULL CHECK (visibility IN ('private', 'public', 'unlisted')),
    status TEXT NOT NULL CHECK (status IN ('importing', 'queued', 'processing', 'ready', 'failed', 'deleting', 'deleted')),
    duration_ms INTEGER,
    width INTEGER,
    height INTEGER,
    video_codec TEXT,
    audio_codec TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    published_at TEXT,
    deleted_at TEXT
);
CREATE INDEX media_files_account_created_idx ON media_files(account_id, created_at DESC);
CREATE INDEX media_files_account_status_idx ON media_files(account_id, status);

CREATE TABLE media_variants (
    id TEXT PRIMARY KEY NOT NULL,
    media_id TEXT NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    width INTEGER NOT NULL CHECK (width > 0),
    height INTEGER NOT NULL CHECK (height > 0),
    bandwidth_bps INTEGER NOT NULL CHECK (bandwidth_bps > 0),
    codecs TEXT NOT NULL,
    playlist_object_key TEXT NOT NULL,
    storage_bytes INTEGER NOT NULL CHECK (storage_bytes >= 0),
    created_at TEXT NOT NULL,
    UNIQUE(media_id, name)
);
CREATE INDEX media_variants_media_id_idx ON media_variants(media_id);

CREATE TABLE media_jobs (
    id TEXT PRIMARY KEY NOT NULL,
    media_id TEXT NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('remote_import', 'transcode', 'delete')),
    status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')),
    payload_json TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    maximum_attempts INTEGER NOT NULL CHECK (maximum_attempts > 0),
    run_after TEXT NOT NULL,
    locked_at TEXT,
    locked_by TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX media_jobs_claim_idx ON media_jobs(status, run_after, created_at);
CREATE INDEX media_jobs_media_id_idx ON media_jobs(media_id, created_at DESC);
CREATE UNIQUE INDEX media_jobs_active_kind_idx ON media_jobs(media_id, kind)
WHERE status IN ('queued', 'running');

CREATE TABLE playback_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    media_id TEXT NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    viewer_hash TEXT NOT NULL,
    started_at TEXT NOT NULL,
    last_event_at TEXT NOT NULL,
    play_started_at TEXT,
    completed_at TEXT,
    last_position_ms INTEGER NOT NULL DEFAULT 0 CHECK (last_position_ms >= 0),
    watched_ms INTEGER NOT NULL DEFAULT 0 CHECK (watched_ms >= 0),
    bytes_served INTEGER NOT NULL DEFAULT 0 CHECK (bytes_served >= 0)
);
CREATE INDEX playback_sessions_media_started_idx ON playback_sessions(media_id, started_at DESC);

CREATE TABLE analytics_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    media_id TEXT NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
    session_id TEXT REFERENCES playback_sessions(id) ON DELETE SET NULL,
    kind TEXT NOT NULL CHECK (kind IN ('view', 'play', 'pause', 'seek', 'heartbeat', 'ended', 'bytes_served')),
    position_ms INTEGER,
    watched_delta_ms INTEGER NOT NULL DEFAULT 0 CHECK (watched_delta_ms >= 0),
    bytes INTEGER NOT NULL DEFAULT 0 CHECK (bytes >= 0),
    occurred_at TEXT NOT NULL
);
CREATE INDEX analytics_events_account_time_idx ON analytics_events(account_id, occurred_at);
CREATE INDEX analytics_events_media_time_idx ON analytics_events(media_id, occurred_at);

CREATE TABLE daily_viewers (
    day TEXT NOT NULL,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    media_id TEXT NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
    viewer_hash TEXT NOT NULL,
    PRIMARY KEY(day, media_id, viewer_hash)
);
CREATE INDEX daily_viewers_account_day_idx ON daily_viewers(account_id, day);

CREATE TABLE daily_stats (
    day TEXT NOT NULL,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    media_id TEXT NOT NULL REFERENCES media_files(id) ON DELETE CASCADE,
    views INTEGER NOT NULL DEFAULT 0 CHECK (views >= 0),
    unique_viewers INTEGER NOT NULL DEFAULT 0 CHECK (unique_viewers >= 0),
    play_starts INTEGER NOT NULL DEFAULT 0 CHECK (play_starts >= 0),
    completed_views INTEGER NOT NULL DEFAULT 0 CHECK (completed_views >= 0),
    watch_time_ms INTEGER NOT NULL DEFAULT 0 CHECK (watch_time_ms >= 0),
    bytes_served INTEGER NOT NULL DEFAULT 0 CHECK (bytes_served >= 0),
    PRIMARY KEY(day, media_id)
);
CREATE INDEX daily_stats_account_day_idx ON daily_stats(account_id, day);
