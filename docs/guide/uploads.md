# Uploads and imports

## Direct upload

Send exactly one `file` field to `POST /api/media`. Optional fields are `title`, `description`, and `visibility`.

```bash
curl -X POST http://localhost:8080/api/media \
  -H "Authorization: Bearer $GULFSTREAM_API_KEY" \
  -F 'file=@trailer.mp4;type=video/mp4' \
  -F 'title=Product trailer' \
  -F 'description=Launch trailer' \
  -F 'visibility=unlisted'
```

```json
{
  "id": "4d65d154-7674-46c1-b87c-e20f4bda2bb0",
  "status": "queued"
}
```

The source is streamed rather than buffered in memory, size-limited, SHA-256 hashed, checked against the account quota in a database write transaction, moved to durable storage, and then queued.

## Remote import

```bash
curl -X POST http://localhost:8080/api/media/imports \
  -H "Authorization: Bearer $GULFSTREAM_API_KEY" \
  -H 'Content-Type: application/json' \
  -d '{
    "url": "https://media.example.com/trailer.mp4",
    "title": "Product trailer",
    "visibility": "private"
  }'
```

Remote import is a durable job. By default Gulfstream rejects private, loopback, link-local, multicast, and otherwise non-public destinations; resolves and pins addresses for each request; revalidates redirects; disables ambient proxies; and enforces configured size, timeout, and redirect limits.
