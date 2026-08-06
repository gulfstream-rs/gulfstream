# Third-party notices

The Gulfstream source code is licensed under the MIT License. The published container image also includes separately licensed third-party software.

## FFmpeg and FFprobe

The container copies the `ffmpeg` and `ffprobe` binaries from the pinned multi-architecture image `mwader/static-ffmpeg:7.1.1`. That build enables GPL and version 3 components, including libx264, and reports GPL version 3 or later licensing terms.

- Upstream project: <https://ffmpeg.org/>
- License information: <https://ffmpeg.org/legal.html>
- Reproducible static-build source: <https://github.com/wader/static-ffmpeg>
- Pinned image digest: `sha256:11a44711684c0b9f754c047dcd64235b8b52deab251bd0e0a86f22faa160749c`

FFmpeg and its libraries are independent works and are not relicensed under Gulfstream's MIT License. Redistributors of the container image are responsible for complying with the applicable FFmpeg and enabled-library licenses.

## BusyBox

The container uses the `wget` and `sh` applets from the pinned official `busybox:1.37.0-musl` image solely for its Docker health check.

- Upstream project: <https://busybox.net/>
- Source: <https://github.com/mirror/busybox>
- License: GPL version 2
- Pinned image digest: `sha256:fc6dddc4c44b1bfe37f41cae8e67d1693828e8f42a91862816d7953e2c9d3f23`

## Distroless runtime

The runtime filesystem is based on Google's pinned `gcr.io/distroless/cc-debian12:nonroot` image and includes Debian runtime libraries, CA certificates, timezone data, and related operating-system files under their respective licenses.

- Project and license data: <https://github.com/GoogleContainerTools/distroless>
- Pinned image digest: `sha256:fccdbb0a547c14e23fcf4ce8ad62ca5d43b4faae8d22cd292f490fef9946c96e`
