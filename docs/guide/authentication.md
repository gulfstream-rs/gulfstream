# Accounts and authentication

## Registration

`registration.mode` controls account creation:

- `open`: anyone can register.
- `admin_token`: the request must include the configured registration header and secret.
- `disabled`: registration returns a permission error.

Passwords are hashed with configurable Argon2id parameters and a server-side pepper. Email addresses are normalized and unique.

## Browser sessions

`POST /api/auth/login` verifies the password, creates a hashed server-side session, sets the configured HttpOnly cookie, and returns a CSRF token. Sessions have both absolute and idle expiration, a configurable per-account cap, and periodic cleanup.

For cookie-authenticated `POST`, `PATCH`, and `DELETE` requests, include the configured CSRF header. `GET /api/auth/session` rotates that token.

## API keys

API integrations use revocable bearer keys:

```http
Authorization: Bearer gfs_…
```

Only a verifier is stored. The plaintext key appears once in the create response. API-key requests do not require a CSRF header.

## Private playback

Owners create a short-lived playback token. The watch endpoint redeems it into media-scoped cookies and redirects to remove the secret from the address bar. Stream routes verify those cookies before serving private playlists or assets.
