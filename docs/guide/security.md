# Security

Production defaults should be reviewed for the deployment environment.

- Terminate HTTPS before or at Gulfstream and set both browser and analytics cookies to `Secure`.
- Generate independent high-entropy values for every secret; never reuse a signing key or pepper.
- Restrict CORS to exact HTTPS origins when credentials are enabled.
- Keep remote private-network access disabled unless the server is intentionally importing from trusted internal endpoints.
- Leave system proxies disabled unless the proxy is trusted and required.
- Run as an unprivileged user and grant write access only to configured database, temporary, storage, and trash locations.
- Restrict registration or disable it after provisioning accounts.
- Rotate API keys and revoke unused credentials.
- Treat playback URLs as temporary bearer secrets and avoid logging query strings.
- Keep Rust, FFmpeg, the base image, and frontend documentation tooling updated through reviewed dependency changes.

See the repository `SECURITY.md` for vulnerability reporting.
