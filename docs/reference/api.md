# API endpoints

The default API prefix is `/api`; every public prefix is configurable. The live server exposes the authoritative OpenAPI 3.1 document at its configured OpenAPI route.

## Authentication

Use a bearer API key for integrations:

```http
Authorization: Bearer gfs_…
```

The browser UI uses a session cookie. State-changing cookie-authenticated calls also send the configured CSRF header.

## Endpoint summary

| Method | Default path | Purpose |
|---|---|---|
| GET | `/health/live` | Process liveness |
| GET | `/health/ready` | Dependency readiness |
| GET | `/openapi.yaml` | Runtime-resolved OpenAPI 3.1 document |
| POST | `/api/accounts` | Register an account |
| POST | `/api/auth/login` | Create browser session |
| GET | `/api/auth/session` | Read/refresh browser session |
| POST | `/api/auth/logout` | Revoke browser session |
| GET/PATCH | `/api/account` | Read/update account |
| GET/POST | `/api/account/api-keys` | List/create API keys |
| DELETE | `/api/account/api-keys/{id}` | Revoke API key |
| GET | `/api/dashboard` | Dashboard snapshot |
| GET/POST | `/api/media` | List/upload media |
| POST | `/api/media/imports` | Queue remote import |
| GET/PATCH/DELETE | `/api/media/{id}` | Read/update/delete media |
| POST | `/api/media/{id}/retry` | Retry failed processing |
| GET | `/api/media/{id}/jobs` | Media processing history |
| POST | `/api/media/{id}/playback-tokens` | Create private watch URL |
| GET | `/api/jobs` | List account jobs |
| GET | `/api/analytics/summary` | Analytics totals |
| GET | `/api/analytics/time-series` | Daily analytics |
| POST | `/api/playback/{session}/events` | Player event ingestion |
| GET | `/watch/{id}` | HTML5 player |
| GET | `/stream/{id}/master.m3u8` | HLS master playlist |
| GET | `/stream/{id}/source` | Original source |
| GET | `/stream/{id}/{variant}/{asset}` | HLS variant assets |

For request fields, status codes, schemas, query parameters, and examples, use [the OpenAPI document](./openapi.md) or [response examples](./responses.md).
