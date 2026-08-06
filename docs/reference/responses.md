# Response examples

## Account

```json
{
  "id": "92442cb8-8ed5-4457-a59e-a6b1a6f91143",
  "email": "developer@example.com",
  "display_name": "Example Developer",
  "status": "active",
  "storage_quota_bytes": 107374182400,
  "storage_used_bytes": 734003200,
  "created_at": "2026-08-06T02:00:00+00:00",
  "updated_at": "2026-08-06T02:00:00+00:00"
}
```

## Browser session

```json
{
  "account": {
    "id": "92442cb8-8ed5-4457-a59e-a6b1a6f91143",
    "email": "developer@example.com",
    "display_name": "Example Developer",
    "status": "active",
    "storage_quota_bytes": 107374182400,
    "storage_used_bytes": 734003200,
    "created_at": "2026-08-06T02:00:00+00:00",
    "updated_at": "2026-08-06T02:00:00+00:00"
  },
  "csrf_token": "browser-csrf-token",
  "expires_at": "2026-08-13T02:00:00+00:00"
}
```


## API key issuance

The plaintext key is returned once. Only its verifier is retained.

```json
{
  "id": "53e7c1f6-e765-4ced-a606-a443c63f84a4",
  "name": "deployment",
  "api_key": "gfs_4f98c8dca4624e95b351a816a2737c43",
  "created_at": "2026-08-06T02:05:00+00:00"
}
```

The list endpoint omits plaintext keys:

```json
[
  {
    "id": "53e7c1f6-e765-4ced-a606-a443c63f84a4",
    "name": "deployment",
    "created_at": "2026-08-06T02:05:00+00:00",
    "last_used_at": "2026-08-06T02:10:00+00:00",
    "revoked_at": null
  }
]
```

## Accepted upload or remote import

Direct uploads and remote imports return `202 Accepted`. Processing continues through durable jobs.

```json
{
  "id": "4d65d154-7674-46c1-b87c-e20f4bda2bb0",
  "status": "queued"
}
```

A remote import initially uses `"status": "importing"`.

## Dashboard

```json
{
  "account": {
    "id": "92442cb8-8ed5-4457-a59e-a6b1a6f91143",
    "email": "developer@example.com",
    "display_name": "Example Developer",
    "status": "active",
    "storage_quota_bytes": 107374182400,
    "storage_used_bytes": 734003200,
    "created_at": "2026-08-06T02:00:00+00:00",
    "updated_at": "2026-08-06T02:00:00+00:00"
  },
  "media_total": 8,
  "media_by_status": [
    {"status": "processing", "count": 1},
    {"status": "ready", "count": 7}
  ],
  "jobs_total": 11,
  "jobs_by_status": [
    {"status": "running", "count": 1},
    {"status": "succeeded", "count": 10}
  ],
  "analytics_from": "2026-07-31",
  "analytics_to": "2026-08-06",
  "analytics": {
    "views": 127,
    "unique_viewers": 95,
    "play_starts": 120,
    "completed_views": 84,
    "watch_time_ms": 6200000,
    "bytes_served": 1843200000
  }
}
```

## Media

```json
{
  "id": "4d65d154-7674-46c1-b87c-e20f4bda2bb0",
  "source_filename": "trailer.mp4",
  "source_mime_type": "video/mp4",
  "source_size_bytes": 734003200,
  "storage_bytes": 1048576000,
  "sha256": "04715fb5f5d51a2ac6ef79dbea4d624a5fc7c693da21be7e0f20bb6a754b5402",
  "title": "Product trailer",
  "description": "Launch trailer",
  "visibility": "unlisted",
  "status": "ready",
  "duration_ms": 60000,
  "width": 1920,
  "height": 1080,
  "video_codec": "h264",
  "audio_codec": "aac",
  "error_message": null,
  "created_at": "2026-08-06T02:00:00+00:00",
  "updated_at": "2026-08-06T02:04:00+00:00",
  "published_at": "2026-08-06T02:04:00+00:00",
  "variants": [
    {"name": "360p", "width": 640, "height": 360, "bandwidth_bps": 952000, "codecs": "avc1.4d401e,mp4a.40.2"},
    {"name": "720p", "width": 1280, "height": 720, "bandwidth_bps": 3124000, "codecs": "avc1.64001f,mp4a.40.2"}
  ]
}
```

## Processing job

```json
{
  "id": "29f8acc8-b4b9-494a-bbba-cf00f35413db",
  "media_id": "4d65d154-7674-46c1-b87c-e20f4bda2bb0",
  "media_title": "Product trailer",
  "kind": "transcode",
  "status": "running",
  "attempts": 1,
  "maximum_attempts": 3,
  "run_after": "2026-08-06T02:00:00+00:00",
  "error_message": null,
  "created_at": "2026-08-06T02:00:00+00:00",
  "updated_at": "2026-08-06T02:01:00+00:00"
}
```

## Analytics summary

```json
{
  "from": "2026-08-01",
  "to": "2026-08-06",
  "media_id": null,
  "totals": {
    "views": 127,
    "unique_viewers": 95,
    "play_starts": 120,
    "completed_views": 84,
    "watch_time_ms": 6200000,
    "bytes_served": 1843200000
  }
}
```


## Paginated media list

```json
{
  "items": [
    {
      "id": "4d65d154-7674-46c1-b87c-e20f4bda2bb0",
      "source_filename": "trailer.mp4",
      "source_mime_type": "video/mp4",
      "source_size_bytes": 734003200,
      "storage_bytes": 1048576000,
      "sha256": "04715fb5f5d51a2ac6ef79dbea4d624a5fc7c693da21be7e0f20bb6a754b5402",
      "title": "Product trailer",
      "description": "Launch trailer",
      "visibility": "unlisted",
      "status": "ready",
      "duration_ms": 60000,
      "width": 1920,
      "height": 1080,
      "video_codec": "h264",
      "audio_codec": "aac",
      "error_message": null,
      "created_at": "2026-08-06T02:00:00+00:00",
      "updated_at": "2026-08-06T02:04:00+00:00",
      "published_at": "2026-08-06T02:04:00+00:00",
      "variants": []
    }
  ],
  "page": 1,
  "page_size": 25,
  "total": 1
}
```

## Paginated processing jobs

```json
{
  "items": [
    {
      "id": "29f8acc8-b4b9-494a-bbba-cf00f35413db",
      "media_id": "4d65d154-7674-46c1-b87c-e20f4bda2bb0",
      "media_title": "Product trailer",
      "kind": "transcode",
      "status": "running",
      "attempts": 1,
      "maximum_attempts": 3,
      "run_after": "2026-08-06T02:00:00+00:00",
      "error_message": null,
      "created_at": "2026-08-06T02:00:00+00:00",
      "updated_at": "2026-08-06T02:01:00+00:00"
    }
  ],
  "page": 1,
  "page_size": 25,
  "total": 1
}
```

## Playback token

```json
{
  "token": "signed-playback-token",
  "expires_in_seconds": 900,
  "watch_url": "https://video.example.com/watch/4d65d154-7674-46c1-b87c-e20f4bda2bb0?token=signed-playback-token"
}
```

## Analytics time series

```json
{
  "from": "2026-08-05",
  "to": "2026-08-06",
  "media_id": null,
  "points": [
    {
      "day": "2026-08-05",
      "views": 61,
      "unique_viewers": 48,
      "play_starts": 58,
      "completed_views": 39,
      "watch_time_ms": 2960000,
      "bytes_served": 870000000
    },
    {
      "day": "2026-08-06",
      "views": 66,
      "unique_viewers": 47,
      "play_starts": 62,
      "completed_views": 45,
      "watch_time_ms": 3240000,
      "bytes_served": 973200000
    }
  ]
}
```

## Empty success responses

Successful logout, API-key revocation, and playback-event ingestion return `204 No Content`. Media deletion and processing retry return `202 Accepted` because durable workers complete those operations asynchronously.

## Error

```json
{
  "error": {
    "code": "bad_request",
    "message": "invalid visibility filter"
  }
}
```
