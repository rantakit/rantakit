# Accept HEVC patent risk for decoding HEIC

Users often receive iPhone photos in HEIC through WhatsApp or Drive, and portals reject them, so converting HEIC to JPG is part of Job 1. HEIC images are compressed with HEVC, which is covered by patents, and every HEIC decoder carries that risk, whether it is a library or the operating system's own. The Windows system decoder also needs a paid extension. We decided to decode HEIC with the pure Rust `heic` crate on every platform and to accept the HEVC patent risk for decoding only. Rantakit never encodes HEVC or HEIC; Results are always JPG or PNG. Android's system decoder stays in reserve in case `heic` does not fit the size budget.

## Consequences

- `heic` is licensed "AGPL-3.0-only OR LicenseRef-Imazen-Commercial". Rantakit's own code stays AGPL-3.0-or-later, but builds that include `heic` (all of them, from v0.4) can be shared only under AGPL-3.0, until `heic` allows later versions or is replaced.
- `heic` applies HEIF's own rotation and mirroring while decoding, which HEIF uses in place of the EXIF orientation, so a HEIC needs no further turning.
- `heic` is used without its `parallel` feature, which would add rayon.
