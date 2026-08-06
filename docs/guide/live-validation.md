# Live validation

This runbook reproduces the manual validation performed against Gulfstream 4.0.0 on August 6, 2026. It covers the CI commands, Docker health, browser and API-key authentication, a real video upload, FFmpeg conversion, remote import, playback, analytics, retry, and deletion.

Run commands from the repository root. The examples use port `18080`, `jq`, `curl`, Docker, and FFmpeg. IDs, timestamps, tokens, hashes, byte counts, and processing durations vary between runs.

## Build the test image

```bash
docker build --tag gulfstream:e2e .
docker image inspect gulfstream:e2e --format '{{.Size}}'
docker history gulfstream:e2e
```

The optimized image should be well below the previous approximately 750 MB unpacked image. It uses pinned multi-architecture static FFmpeg and FFprobe binaries instead of Debian's complete FFmpeg dependency graph.

Confirm the required media capabilities:

```bash
docker run --rm --entrypoint /usr/bin/ffmpeg gulfstream:e2e \
  -hide_banner -encoders 2>&1 | grep -E 'libx264| AAC '
docker run --rm --entrypoint /usr/bin/ffmpeg gulfstream:e2e \
  -hide_banner -muxers 2>&1 | grep -E ' hls | mp4 '
docker run --rm --entrypoint /usr/bin/ffprobe gulfstream:e2e -version
```

## Start an isolated server

Create temporary test secrets. Do not reuse them in production.

```bash
./scripts/generate-secrets.sh > .env
docker volume create gulfstream-e2e-data
docker run --detach \
  --name gulfstream-e2e \
  --publish 18080:8080 \
  --env-file .env \
  --volume gulfstream-e2e-data:/app/data \
  gulfstream:e2e
```

Wait for Docker and application readiness:

```bash
docker inspect --format '{{.State.Status}} {{.State.Health.Status}}' gulfstream-e2e
curl --fail http://127.0.0.1:18080/health/live
curl --fail http://127.0.0.1:18080/health/ready
curl --fail --output /dev/null --write-out '%{http_code}\n' \
  http://127.0.0.1:18080/openapi.yaml
```

Expected responses:

```text
running healthy
{"status":"ok"}
{"status":"ready"}
200
```

Set reusable paths and read the generated registration token:

```bash
export BASE_URL=http://127.0.0.1:18080
export ADMIN_TOKEN="$(grep '^GULFSTREAM_ADMIN_TOKEN=' .env | cut -d= -f2-)"
export TEST_ROOT="$(mktemp -d)"
```

## Check web routes

The root redirects to the canonical dashboard URL, `/app` without a trailing slash.

```bash
curl --output /dev/null --write-out '%{http_code} %{redirect_url}\n' "$BASE_URL/"
for path in app app/register app/login app/upload app/media app/jobs app/analytics app/account; do
  curl --fail --output /dev/null --write-out "$path %{http_code}\n" "$BASE_URL/$path"
done
curl --fail --output /dev/null "$BASE_URL/app/assets/app.js"
curl --fail --output /dev/null "$BASE_URL/app/assets/styles/base.css"
```

Expected status codes are `303` for `/` and `200` for each canonical page and asset.

## Register and authenticate

Registration without the configured admin token should be rejected:

```bash
curl --output /dev/null --write-out '%{http_code}\n' \
  --request POST "$BASE_URL/api/accounts" \
  --header 'content-type: application/json' \
  --data '{"email":"e2e@example.com","display_name":"E2E User","password":"correct-horse-battery"}'
```

Expected response: `403`.

Register the account:

```bash
curl --fail --request POST "$BASE_URL/api/accounts" \
  --header 'content-type: application/json' \
  --header "x-gulfstream-registration-token: $ADMIN_TOKEN" \
  --data '{"email":"e2e@example.com","display_name":"E2E User","password":"correct-horse-battery"}' | jq .
```

Representative response:

```json
{
  "id": "f7eb86b5-1aa0-406c-97d3-a9a146828f97",
  "email": "e2e@example.com",
  "display_name": "E2E User",
  "status": "active",
  "storage_quota_bytes": 107374182400,
  "storage_used_bytes": 0,
  "created_at": "2026-08-06T15:55:30Z",
  "updated_at": "2026-08-06T15:55:30Z"
}
```

Log in and refresh the session:

```bash
curl --fail --cookie-jar "$TEST_ROOT/browser.cookies" \
  --output "$TEST_ROOT/login.json" \
  --request POST "$BASE_URL/api/auth/login" \
  --header 'content-type: application/json' \
  --data '{"email":"e2e@example.com","password":"correct-horse-battery"}'

curl --fail --cookie "$TEST_ROOT/browser.cookies" \
  --output "$TEST_ROOT/session.json" \
  "$BASE_URL/api/auth/session"
export CSRF="$(jq -r .csrf_token "$TEST_ROOT/session.json")"
jq '{email: .account.email, csrf_length: (.csrf_token | length), expires_at}' \
  "$TEST_ROOT/session.json"
```

`GET /api/auth/session` rotates the CSRF token. Use the new token for subsequent browser-authenticated writes.

Update and read the account:

```bash
curl --fail --cookie "$TEST_ROOT/browser.cookies" \
  --request PATCH "$BASE_URL/api/account" \
  --header 'content-type: application/json' \
  --header "x-gulfstream-csrf: $CSRF" \
  --data '{"display_name":"E2E Updated"}' | jq '{email, display_name}'

curl --fail --cookie "$TEST_ROOT/browser.cookies" "$BASE_URL/api/account" | jq .
```

Create an API key and use bearer authentication for the remaining owner API calls:

```bash
curl --fail --cookie "$TEST_ROOT/browser.cookies" \
  --output "$TEST_ROOT/api-key.json" \
  --request POST "$BASE_URL/api/account/api-keys" \
  --header 'content-type: application/json' \
  --header "x-gulfstream-csrf: $CSRF" \
  --data '{"name":"E2E Key"}'
export API_KEY="$(jq -r .api_key "$TEST_ROOT/api-key.json")"
export API_KEY_ID="$(jq -r .id "$TEST_ROOT/api-key.json")"

curl --fail --header "authorization: Bearer $API_KEY" \
  "$BASE_URL/api/account/api-keys" | jq .
curl --fail --header "authorization: Bearer $API_KEY" \
  "$BASE_URL/api/dashboard" | jq '{media_total, jobs_total, analytics}'
```

## Generate and upload a real video

The original live run used a 17 MiB, 30.527-second, 1920x1080 H.264/AAC file. Use `video.mp4` if it is available locally, or generate a shorter equivalent fixture:

```bash
ffmpeg -hide_banner -y \
  -f lavfi -i 'testsrc2=size=1920x1080:rate=30' \
  -f lavfi -i 'sine=frequency=1000:sample_rate=48000' \
  -t 8 -c:v libx264 -pix_fmt yuv420p -c:a aac -shortest video.mp4
ffprobe -v error -show_entries stream=codec_name,width,height \
  -show_entries format=duration,size -of json video.mp4 | jq .
```

Upload it as private media:

```bash
curl --fail --header "authorization: Bearer $API_KEY" \
  --output "$TEST_ROOT/upload.json" \
  --form 'title=E2E Uploaded Video' \
  --form 'description=Actual 1080p upload conversion test' \
  --form 'visibility=private' \
  --form 'file=@video.mp4;type=video/mp4' \
  "$BASE_URL/api/media"
export MEDIA_ID="$(jq -r .id "$TEST_ROOT/upload.json")"
jq . "$TEST_ROOT/upload.json"
```

Initial response:

```json
{"id":"8c5f7d01-9247-4b8e-957f-2a8ee42e0447","status":"queued"}
```

Poll until processing finishes:

```bash
while true; do
  curl --fail --silent --header "authorization: Bearer $API_KEY" \
    "$BASE_URL/api/media/$MEDIA_ID" > "$TEST_ROOT/media.json"
  jq '{status, error_message, duration_ms, width, height, variants}' "$TEST_ROOT/media.json"
  status="$(jq -r .status "$TEST_ROOT/media.json")"
  if [ "$status" = ready ] || [ "$status" = failed ]; then
    break
  fi
  sleep 5
test "$(jq -r .status "$TEST_ROOT/media.json")" = ready
```

The 1080p live fixture produced:

```json
{
  "status": "ready",
  "duration_ms": 30527,
  "width": 1920,
  "height": 1080,
  "variants": [
    {"name":"360p","width":640,"height":360,"bandwidth_bps":952000,"codecs":"avc1.4d401e,mp4a.40.2"},
    {"name":"720p","width":1280,"height":720,"bandwidth_bps":3124000,"codecs":"avc1.64001f,mp4a.40.2"},
    {"name":"1080p","width":1920,"height":1080,"bandwidth_bps":5542000,"codecs":"avc1.640028,mp4a.40.2"}
  ]
}
```

Exercise metadata, filtering, and job history:

```bash
curl --fail --header "authorization: Bearer $API_KEY" \
  --request PATCH "$BASE_URL/api/media/$MEDIA_ID" \
  --header 'content-type: application/json' \
  --data '{"title":"E2E Converted Video","description":"Converted and stream tested","visibility":"private"}' | jq .

curl --fail --header "authorization: Bearer $API_KEY" \
  "$BASE_URL/api/media?status=ready&visibility=private&search=Converted&page=1&page_size=25" | jq .
curl --fail --header "authorization: Bearer $API_KEY" \
  "$BASE_URL/api/media/$MEDIA_ID/jobs" | jq .
curl --fail --header "authorization: Bearer $API_KEY" \
  "$BASE_URL/api/jobs?status=succeeded&kind=transcode&page=1&page_size=25" | jq .
```

The media job should report `kind: "transcode"`, `status: "succeeded"`, and `attempts: 1`.

## Validate private playback

Private playback must reject requests without credentials:

```bash
curl --output /dev/null --write-out '%{http_code}\n' "$BASE_URL/watch/$MEDIA_ID"
curl --output /dev/null --write-out '%{http_code}\n' "$BASE_URL/stream/$MEDIA_ID/master.m3u8"
```

Both responses should be `401`.

Create and redeem a playback token:

```bash
curl --fail --header "authorization: Bearer $API_KEY" \
  --output "$TEST_ROOT/playback-token.json" \
  --request POST "$BASE_URL/api/media/$MEDIA_ID/playback-tokens"
export PLAYBACK_TOKEN="$(jq -r .token "$TEST_ROOT/playback-token.json")"

curl --cookie-jar "$TEST_ROOT/playback.cookies" \
  --output /dev/null --write-out '%{http_code} %{redirect_url}\n' \
  "$BASE_URL/watch/$MEDIA_ID?token=$PLAYBACK_TOKEN"
curl --fail --cookie "$TEST_ROOT/playback.cookies" \
  --cookie-jar "$TEST_ROOT/playback.cookies" \
  --output "$TEST_ROOT/player.html" "$BASE_URL/watch/$MEDIA_ID"
```

Token redemption returns `303`, removes the token from the redirect URL, and establishes scoped playback cookies. The subsequent watch request returns HTML with the stream and event URLs.

Fetch the converted assets and original source range:

```bash
curl --fail --cookie "$TEST_ROOT/playback.cookies" \
  --output "$TEST_ROOT/master.m3u8" "$BASE_URL/stream/$MEDIA_ID/master.m3u8"
curl --fail --cookie "$TEST_ROOT/playback.cookies" \
  --output "$TEST_ROOT/360p.m3u8" "$BASE_URL/stream/$MEDIA_ID/360p/index.m3u8"
curl --fail --cookie "$TEST_ROOT/playback.cookies" \
  --output /dev/null --write-out '%{http_code} %{content_type} %{size_download}\n' \
  "$BASE_URL/stream/$MEDIA_ID/360p/init.mp4"
curl --fail --cookie "$TEST_ROOT/playback.cookies" \
  --output /dev/null --write-out '%{http_code} %{content_type} %{size_download}\n' \
  "$BASE_URL/stream/$MEDIA_ID/360p/segment_000000.m4s"
curl --fail --cookie "$TEST_ROOT/playback.cookies" \
  --header 'range: bytes=0-1023' --output /dev/null \
  --write-out '%{http_code} %{content_type} %{size_download}\n' \
  "$BASE_URL/stream/$MEDIA_ID/source"
```

Representative results:

```text
200 video/mp4 1408
200 video/iso.segment 440259
206 video/mp4 1024
```

## Record playback analytics

Read the playback session ID from the cookie jar and submit real player events:

```bash
export PLAYBACK_SESSION_ID="$(awk '$6 == "gulfstream_playback_session" {print $7}' "$TEST_ROOT/playback.cookies")"
for payload in \
  '{"kind":"play","position_ms":0,"watched_delta_ms":0}' \
  '{"kind":"heartbeat","position_ms":10000,"watched_delta_ms":10000}' \
  '{"kind":"pause","position_ms":10000,"watched_delta_ms":0}' \
  '{"kind":"seek","position_ms":25000,"watched_delta_ms":0}' \
  '{"kind":"ended","position_ms":30527,"watched_delta_ms":5000}'
do
  curl --fail --cookie "$TEST_ROOT/playback.cookies" --output /dev/null \
    --write-out '%{http_code}\n' \
    --request POST "$BASE_URL/api/playback/$PLAYBACK_SESSION_ID/events" \
    --header 'content-type: application/json' --data "$payload"
done
```

Each event returns `204`. Query the persisted totals and daily points:

```bash
export TODAY="$(date -u +%F)"
curl --fail --header "authorization: Bearer $API_KEY" \
  "$BASE_URL/api/analytics/summary?from=$TODAY&to=$TODAY&media_id=$MEDIA_ID" | jq .
curl --fail --header "authorization: Bearer $API_KEY" \
  "$BASE_URL/api/analytics/time-series?from=$TODAY&to=$TODAY&media_id=$MEDIA_ID" | jq .
```

Expected counters include one view, one unique viewer, one play start, and one completed view. Watch time is constrained by real elapsed wall time and may be lower than the submitted deltas. Requests for inactive dates still return one zero-valued point per requested day.

## Validate public and unlisted playback

Change the uploaded media to public and confirm anonymous playback:

```bash
curl --fail --header "authorization: Bearer $API_KEY" \
  --request PATCH "$BASE_URL/api/media/$MEDIA_ID" \
  --header 'content-type: application/json' \
  --data '{"visibility":"public"}' | jq '{id, visibility, status}'
curl --fail --output /dev/null "$BASE_URL/watch/$MEDIA_ID"
curl --fail --output /dev/null "$BASE_URL/stream/$MEDIA_ID/master.m3u8"
curl --fail --header 'range: bytes=-512' --output /dev/null \
  --write-out '%{http_code} %{size_download}\n' "$BASE_URL/stream/$MEDIA_ID/source"
```

The suffix range should return `206 512`.

## Validate remote import

Submit the same real public MP4 used during the live run:

```bash
curl --fail --header "authorization: Bearer $API_KEY" \
  --output "$TEST_ROOT/import.json" \
  --request POST "$BASE_URL/api/media/imports" \
  --header 'content-type: application/json' \
  --data '{"url":"https://samplelib.com/lib/preview/mp4/sample-5s.mp4","title":"E2E Remote Import","description":"Real HTTPS import","visibility":"unlisted"}'
export IMPORT_ID="$(jq -r .id "$TEST_ROOT/import.json")"
jq . "$TEST_ROOT/import.json"
```

The initial status is `importing`. Poll `GET /api/media/$IMPORT_ID` as in the upload section. The live request downloaded 2,848,208 bytes, detected 5.759 seconds of 1920x1080 media, and generated all three configured renditions. Its job history contained successful `remote_import` and `transcode` jobs.

Confirm anonymous unlisted playback:

```bash
curl --fail --output /dev/null "$BASE_URL/watch/$IMPORT_ID"
curl --fail --output /dev/null "$BASE_URL/stream/$IMPORT_ID/master.m3u8"
```

## Validate failure and retry

Upload a non-video payload using an accepted fallback content type:

```bash
curl --fail --header "authorization: Bearer $API_KEY" \
  --output "$TEST_ROOT/invalid-upload.json" \
  --form 'title=E2E Invalid Video' \
  --form 'visibility=private' \
  --form 'file=@README.md;type=application/octet-stream' \
  "$BASE_URL/api/media"
export INVALID_ID="$(jq -r .id "$TEST_ROOT/invalid-upload.json")"
```

Poll the media and jobs endpoints. With the example configuration, FFprobe fails three times using 30- and 60-second backoffs before the media reaches `failed`:

```json
{
  "status": "failed",
  "error_message": "run ffprobe: media command failed ... Invalid data found when processing input"
}
```

Retry and then cancel the new queued attempt through deletion:

```bash
curl --fail --output /dev/null --write-out '%{http_code}\n' \
  --header "authorization: Bearer $API_KEY" \
  --request POST "$BASE_URL/api/media/$INVALID_ID/retry"
curl --fail --output /dev/null --write-out '%{http_code}\n' \
  --header "authorization: Bearer $API_KEY" \
  --request DELETE "$BASE_URL/api/media/$INVALID_ID"
```

Both requests return `202`. After the deletion worker completes, `GET /api/media/$INVALID_ID` returns `404` and the delete job reports `succeeded`.

## Delete media and revoke authentication

```bash
for id in "$MEDIA_ID" "$IMPORT_ID"; do
  curl --fail --output /dev/null --write-out '%{http_code}\n' \
    --header "authorization: Bearer $API_KEY" \
    --request DELETE "$BASE_URL/api/media/$id"
done
sleep 5
curl --fail --header "authorization: Bearer $API_KEY" \
  "$BASE_URL/api/jobs?kind=delete&page=1&page_size=25" | jq .

curl --fail --cookie "$TEST_ROOT/browser.cookies" --output /dev/null \
  --write-out '%{http_code}\n' \
  --header "x-gulfstream-csrf: $CSRF" \
  --request DELETE "$BASE_URL/api/account/api-keys/$API_KEY_ID"
curl --output /dev/null --write-out '%{http_code}\n' \
  --header "authorization: Bearer $API_KEY" "$BASE_URL/api/account"

curl --fail --cookie "$TEST_ROOT/browser.cookies" --output /dev/null \
  --write-out '%{http_code}\n' \
  --header "x-gulfstream-csrf: $CSRF" \
  --request POST "$BASE_URL/api/auth/logout"
curl --output /dev/null --write-out '%{http_code}\n' \
  --cookie "$TEST_ROOT/browser.cookies" "$BASE_URL/api/auth/session"
```

Expected final status sequence:

```text
202
202
204
401
204
401
```

## Reproduce the submission gates

Run the same local commands used by GitHub Actions:

```bash
node scripts/validate-web.mjs
(cd docs && npm ci --no-audit --no-fund && npm run docs:build)
cargo metadata --no-deps --format-version 1
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --all-features
cargo build --release
cargo publish --dry-run
cargo deny check
actionlint .github/workflows/*.yml
```

Validate that both release architectures resolve and can build:

```bash
docker buildx build --platform linux/amd64 --load --tag gulfstream:e2e-amd64 .
docker buildx build --platform linux/arm64 --load --tag gulfstream:e2e-arm64 .
```

The publishing workflow creates a single GHCR manifest containing `linux/amd64` and `linux/arm64` images. After a release, inspect it with:

```bash
docker buildx imagetools inspect ghcr.io/gulfstream-rs/gulfstream:4.0.0
```

## Cleanup

```bash
docker rm --force gulfstream-e2e
docker volume rm gulfstream-e2e-data
rm -rf "$TEST_ROOT"
```
