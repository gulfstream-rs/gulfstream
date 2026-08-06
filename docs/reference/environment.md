# Environment variables

Gulfstream reads one required configuration path and expands `${NAME}` placeholders inside TOML basic strings.

| Variable | Purpose |
|---|---|
| `GULFSTREAM_CONFIG` | Path to the active TOML file; may be replaced by `--config` |
| `GULFSTREAM_ADMIN_TOKEN` | Registration secret when mode is `admin_token` |
| `GULFSTREAM_PASSWORD_PEPPER` | Server-side password pepper |
| `GULFSTREAM_SESSION_SIGNING_KEY` | Browser-session and CSRF verifier key |
| `GULFSTREAM_API_KEY_PEPPER` | API-key verifier pepper |
| `GULFSTREAM_PLAYBACK_SIGNING_KEY` | Private playback-token signer |
| `GULFSTREAM_VIEWER_HASH_KEY` | Key for privacy-preserving viewer identifiers |

The names above are conventions used by the example configuration. Placeholders can refer to differently named variables when the TOML file is customized.

`docker-compose.yml` also accepts `GULFSTREAM_PORT`, `GULFSTREAM_DATA_DIRECTORY`, and `GULFSTREAM_CONFIG_FILE` for host deployment paths.
