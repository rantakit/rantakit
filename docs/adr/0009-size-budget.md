# Size budget for 1.0

The size gate ([RANTAKIT.md §7](../../RANTAKIT.md#app-size)) needed real numbers. On 2026-11-01 the size experiment built v0.1.0 once without extras and once per heavy-format library, on every platform, from a throwaway `spike/size` branch ([ARCHITECTURE.md](../../ARCHITECTURE.md#size-experiment)). Sizes are download sizes in MB (10⁶ bytes), with the increase over that platform's baseline in brackets. Android APKs are unsigned release builds.

| Feature | Android arm64 APK | Android armv7 APK | Windows installer | Linux `.deb` |
|---|---:|---:|---:|---:|
| Baseline (v0.1.0) | 7.66 | 6.00 | 2.07 | 3.42 |
| HEIC (`heic`) | 7.97 (+0.32) | 6.20 (+0.20) | 2.21 (+0.14) | 3.61 (+0.19) |
| WebP input (`image-webp`) | 7.83 (+0.17) | 6.11 (+0.12) | 2.14 (+0.07) | 3.52 (+0.10) |
| Office compression (`zip`) | 7.95 (+0.30) | 6.22 (+0.23) | 2.17 (+0.10) | 3.58 (+0.16) |
| Audio (`symphonia`, libopus, LAME) | 8.96 (+1.30) | 7.04 (+1.05) | 2.55 (+0.48) | 4.09 (+0.67) |
| PDF to JPG (`hayro`, default features) | 11.95 (+4.29) | 9.20 (+3.20) | 3.79 (+1.72) | 5.84 (+2.42) |
| PDF to JPG (`mupdf`) | build failed | build failed | 5.23 (+3.16) | 6.38 (+2.97) |
| Word and Excel to PDF (`office2pdf`, no fonts) | 45.84 (+38.18) | 35.98 (+29.98) | 13.27 (+11.19) | 20.26 (+16.84) |

Video was not measured: it uses the platform's own encoders ([ADR 0008](0008-h264-through-native-encoders-only.md)), which add almost nothing.

We decided the 1.0 app must fit these download-size caps:

| Android arm64 APK | Android armv7 APK | Windows installer | Linux `.deb` |
|---:|---:|---:|---:|
| 20 MB | 16 MB | 8 MB | 10 MB |

Every measured format except Word and Excel to PDF fits together with room to spare (about 14.0, 10.8, 4.6, and 6.6 MB), leaving about 5 MB on Android for what later releases add (bundled UI fonts, Scan Dokumen, watermarking, translations).

## Consequences

- **Word and Excel to PDF stays before 1.0** but does not fit yet. Its epic starts by slimming `office2pdf` (unused Typst parts, subset fonts, anything a release build can drop) and measuring again. Only if it still cannot fit do we choose between raising the caps and moving the Tool to after 1.0.
- **PDF to JPG uses `hayro` only.** `mupdf` is bigger on every platform and does not build for Android as published: its build reads the host's Linux headers instead of the NDK's. If `hayro` fails real documents, the fallback is Android's `PdfRenderer` plus a fix to `hayro`.
- **`hayro` without its embedded CJK character maps** is measured at the start of the PDF to JPG epic, since Indonesian documents rarely need them; +4.29 MB is its worst case. Measured on 2026-11-07 for page thumbnails (v0.4, X1), with `default-features = false` and only `embed-fonts`: the Android arm64 library grows by 1.78 MB deflate-compressed (4.10 MB raw), so about +1.8 MB on the APK, and the Windows executable by 2.10 MB gzip-compressed, an upper bound for the installer.
- **Scan Dokumen's camera** (CameraX `camera-camera2`, `camera-lifecycle`, and `camera-view` 1.4.2), with "Bersihkan" and the Scan Dokumen screens, measured on 2026-11-07 as unsigned arm64 release APKs before and after (v0.4, D3 and D4): 13.22 to 13.60 MB, +0.38 MB. CameraX is Java code, so the armv7 APK grows by about the same.
- **Beri Watermark and Hapus Info Lokasi** (v0.4, K1 and K2; `ab_glyph` and a 29.7 KB Latin font) took the unsigned arm64 release APK from 13.60 to 13.88 MB, +0.28 MB, read from the same builds as the next line.
- **HEIC and WebP** (v0.4, O1; `heic` 0.1.6 without `parallel`, and `image`'s `webp` feature), measured on 2026-11-08 as unsigned arm64 release APKs before and after: 13.88 to 14.61 MB, +0.74 MB. That is more than the size experiment's +0.49 MB for the two, and still well inside the cap. The armv7 APK was not measured.
- **Video** passes the size gate. Whether it works (MediaCodec through the NDK, a system encoder on Linux including in Flatpak) is prototyped at the start of its epic.
- Every C library kept here (libopus, LAME) builds from source for all targets, as [ADR 0005](0005-dependency-policy.md) requires.
