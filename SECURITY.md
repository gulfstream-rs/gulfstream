# Security policy

## Reporting a vulnerability

Please report vulnerabilities privately through the security-advisory feature of the [Gulfstream repository](https://github.com/gulfstream-rs/gulfstream/security/advisories/new). Do not include active credentials, private media, or exploit details in a public issue.

Include the affected version, deployment context, reproduction steps, impact, and any proposed mitigation. Maintainers should acknowledge a complete report, investigate it, coordinate a fix, and publish an advisory when appropriate.

## Deployment baseline

- Serve Gulfstream only through HTTPS in production.
- Set `browser_auth.session_cookie_secure` and `analytics.cookie_secure` to `true` behind HTTPS.
- Use independent random values of at least 32 bytes for every pepper and signing key.
- Store secrets in an environment-specific secret manager rather than source control.
- Restrict credentialed CORS to exact trusted origins.
- Keep `remote_imports.allow_private_networks` and `remote_imports.use_system_proxy` disabled unless the deployment explicitly requires and secures them.
- Restrict registration with an administrator token or disable it after provisioning.
- Run the process as an unprivileged account with access only to configured database and storage paths.
- Keep Rust dependencies, FFmpeg, the operating-system image, Node.js, and VitePress updated through reviewed changes.
- Back up SQLite and media storage together and test restores.
- Monitor readiness, failed jobs, disk usage, session cleanup, and unusual playback-token activity.
- Revoke and replace any credential that may have entered logs, shell history, analytics, or source control.

## Security properties

- Account passwords use configurable Argon2id parameters and a server-side pepper.
- API keys are returned once and stored as Argon2id verifiers with a server-side pepper.
- Browser sessions and CSRF values are stored as keyed hashes and expire by absolute and idle deadlines.
- State-changing browser-session requests require a CSRF token; bearer API-key requests do not depend on cookies.
- Private playback uses short-lived signed tokens redeemed into path-scoped cookies.
- Request tracing records URL paths rather than complete URIs, avoiding query-token disclosure.
- Uploads and imports are streamed with configured limits and quota checks.
- Remote imports validate resolved addresses and every redirect, protect against DNS rebinding, and disable ambient proxies by default.
- Filesystem components are validated before use and storage object keys are constrained beneath configured roots.
- Durable jobs use leases, renewal, retries, cancellation, and final publication fencing.
