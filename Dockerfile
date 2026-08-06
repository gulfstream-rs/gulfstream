# syntax=docker/dockerfile:1.7
ARG RUST_IMAGE=rust:1.97.1-bookworm@sha256:14bc9c5966e7b3a385794b3d5389a8765668342025fbcc7b2e3d2866ac4bd8c3
ARG FFMPEG_IMAGE=mwader/static-ffmpeg:7.1.1@sha256:11a44711684c0b9f754c047dcd64235b8b52deab251bd0e0a86f22faa160749c
ARG BUSYBOX_IMAGE=busybox:1.37.0-musl@sha256:fc6dddc4c44b1bfe37f41cae8e67d1693828e8f42a91862816d7953e2c9d3f23
ARG RUNTIME_IMAGE=gcr.io/distroless/cc-debian12:nonroot@sha256:fccdbb0a547c14e23fcf4ce8ad62ca5d43b4faae8d22cd292f490fef9946c96e

FROM ${FFMPEG_IMAGE} AS media-tools
FROM ${BUSYBOX_IMAGE} AS busybox

FROM ${RUST_IMAGE} AS builder
WORKDIR /build

COPY Cargo.toml rust-toolchain.toml README.md CHANGELOG.md CONTRIBUTING.md SECURITY.md LICENSE ./
COPY migrations ./migrations
COPY src ./src
COPY templates ./templates
COPY web ./web
COPY openapi.yaml ./openapi.yaml

RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/build/target,sharing=locked \
    cargo build --release \
    && cp /build/target/release/gulfstream /tmp/gulfstream \
    && mkdir -p /tmp/runtime-data/storage /tmp/runtime-data/tmp

FROM ${RUNTIME_IMAGE} AS runtime
ARG GULFSTREAM_UID=10001
ARG GULFSTREAM_GID=10001

WORKDIR /app
COPY --from=builder /tmp/gulfstream /usr/local/bin/gulfstream
COPY --from=media-tools /ffmpeg /usr/bin/ffmpeg
COPY --from=media-tools /ffprobe /usr/bin/ffprobe
COPY --from=busybox /bin/busybox /usr/local/bin/busybox
COPY config/gulfstream.example.toml /etc/gulfstream/gulfstream.toml
COPY THIRD_PARTY_NOTICES.md /usr/share/doc/gulfstream/THIRD_PARTY_NOTICES.md
COPY templates ./templates
COPY web ./web
COPY --from=builder --chown=${GULFSTREAM_UID}:${GULFSTREAM_GID} /tmp/runtime-data /app/data

USER ${GULFSTREAM_UID}:${GULFSTREAM_GID}
ENV GULFSTREAM_CONFIG=/etc/gulfstream/gulfstream.toml \
    GULFSTREAM_HEALTHCHECK_URL=http://127.0.0.1:8080/health/ready
EXPOSE 8080
VOLUME ["/app/data"]
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD ["/usr/local/bin/busybox", "sh", "-c", "/usr/local/bin/busybox wget --spider -q \"${GULFSTREAM_HEALTHCHECK_URL}\""]
ENTRYPOINT ["/usr/local/bin/gulfstream"]
