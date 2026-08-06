# Processing workflow

Gulfstream uses a durable media-job table rather than an in-memory queue.

## Job kinds

- `remote_import`: download, validate, hash, quota-check, and publish a remote source.
- `transcode`: probe and create adaptive HLS renditions.
- `delete`: remove or move media storage according to retention configuration.

## Job states

- `queued`: eligible after `run_after`.
- `running`: owned by a worker lease.
- `succeeded`: publication completed.
- `failed`: attempts exhausted or explicitly failed.
- `cancelled`: superseded or cancelled by deletion.

## Adaptive HLS

Each `[[transcoding.profiles]]` block controls rendition name, maximum height, video/audio codecs, target/max/buffer bitrates, preset, codec profile, pixel format, frame rate, and RFC 6381 codec strings. Global settings control segment duration, fMP4 or MPEG-TS, HLS version, playlist names, segment patterns, scaling, threads, and extra FFmpeg arguments.

Profiles above the source resolution are skipped unless `allow_upscale` is enabled. Segment boundaries use aligned GOP settings so adaptive switching remains predictable. Outputs are generated in staging and atomically published only after the current worker lease and media state are rechecked.

## Retry behavior

Workers renew leases while FFmpeg or network work runs. Failed work is retried using configured maximum attempts and base delay. The media retry endpoint creates a fresh eligible job for failed media.
