# Quick start

Gulfstream requires FFmpeg and FFprobe. The fastest path is Docker Compose.

## Docker Compose

```bash
# Clone and enter the repository
git clone https://github.com/gulfstream-rs/gulfstream.git
cd gulfstream

# Create the active configuration and a complete local environment file
cp config/gulfstream.example.toml config/gulfstream.toml
{
  printf '%s\n' 'GULFSTREAM_CONFIG=config/gulfstream.toml'
  ./scripts/generate-secrets.sh
} > .env

# Build and start the API, web interface, workers, and SQLite database
docker compose up --build
```

Open:

- Web interface: `http://localhost:8080/app`
- OpenAPI document: `http://localhost:8080/openapi.yaml`
- Readiness: `http://localhost:8080/health/ready`

The example configuration uses administrator-token registration. Enter `GULFSTREAM_ADMIN_TOKEN` from `.env` on the registration page. Change `registration.mode` to `open` for unrestricted registration or `disabled` to prohibit new accounts.

## Run from source

Install Rust 1.97, FFmpeg, FFprobe, and SQLite-compatible build prerequisites, then:

```bash
cp config/gulfstream.example.toml config/gulfstream.toml
./scripts/generate-secrets.sh > .env
set -a; . ./.env; set +a
cargo run --release -- --config config/gulfstream.toml
```

## First API key

1. Register and sign in through `/app`.
2. Open **Account**.
3. Create an API key and copy it immediately.
4. Use it as `Authorization: Bearer <key>` for API integrations.

```bash
curl -H "Authorization: Bearer $GULFSTREAM_API_KEY" \
  http://localhost:8080/api/dashboard
```

## Production checklist

Before exposing Gulfstream publicly, use HTTPS, set both cookie `secure` settings to `true`, restrict CORS to exact origins, keep private-network imports disabled, place secrets in a secret manager, back up the database and storage together, and monitor the readiness endpoint and failed jobs.
