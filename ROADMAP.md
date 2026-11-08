# Rantakit roadmap

This is the order in which Rantakit grows from v0.1.0 to 1.0 and beyond. It lists releases and their epics, not stories, and it has no dates: each release ships when its "Done when" list is true.

Related documents:

- [RANTAKIT.md](RANTAKIT.md): the product plan this roadmap delivers (Tools, UX rules, platforms, design system)
- [GLOSSARY.md](GLOSSARY.md): the words we use (Tool, Task, Original, Result, Size limit, Preset, History, …)
- [ARCHITECTURE.md](ARCHITECTURE.md): the code map, including the size experiment

## How this roadmap works

- **Small releases, except v0.4.** Every release is usable on its own and ships through the existing release workflow. v0.4 is the one large release: it holds the Tools for Jobs 2, 3, and 5 and the remaining photo and document formats.
- **No dates.** Only the order and the contents of releases are planned.
- **Both platforms from v0.4.** v0.2 is desktop only and v0.3 brings Android up to the same level. From v0.4 on, every Tool ships on Windows, Linux, and Android in the same release.
- **One usability test, at the beta.** Five people from the primary audience test the beta of v0.8, before 1.0 (RANTAKIT.md §9). Releases before it do not wait for a usability test.
- **Every Tool epic starts with a short grilling** that decides that Tool's Advanced options and its error messages, before any work on it.
- **The size gate.** Heavy formats (photo formats, office documents, audio, video, Word/Excel to PDF) are measured before they are planned in detail; see [The size gate](#the-size-gate).
- **1.0** means the full catalogue of v0.2 to v0.8 works on Windows, Android, and Linux. Whether 1.0 also requires the store and distribution channels is not decided yet.

Each release below has a **Goal** (what users can do afterwards), **Epics**, **Done when** (observable exit criteria), and **Open decisions** (settled at the start of the epic, not now).

## Overview

| Version | Release | Platforms |
|---|---|---|
| v0.2 | Foundation | Windows, Linux |
| v0.3 | Android | Android |
| v0.4 | More Tools: arrange PDF, paper → PDF, share safely, more formats | All |
| v0.5 | Audio | All |
| v0.6 | Video | All |
| v0.7 | Word/Excel to PDF | All |
| v0.8 | Ready for 1.0 | All |
| 1.0 | Rantakit 1.0 | All |
| After 1.0 | Get text, then protect PDF | All |

## The size gate

Users download Rantakit over prepaid mobile data, so the app must stay small ([RANTAKIT.md §7](RANTAKIT.md#app-size)). The size experiment measured every heavy format on Windows, Android, and Linux, and set the 1.0 caps on download size ([ADR 0009](docs/adr/0009-size-budget.md)):

| Android arm64 APK | Android armv7 APK | Windows installer | Linux `.deb` |
|---:|---:|---:|---:|
| 20 MB | 16 MB | 8 MB | 10 MB |

- Every heavy format fits within the caps except **Word/Excel to PDF**, which stays before 1.0: its epic starts by slimming it to fit (see [v0.7](#v07-wordexcel-to-pdf)).
- A format that still does not fit **moves to after 1.0** until a lighter way to build it is found, unless the caps are raised on purpose. Its release is dropped from this list and the later version numbers close up; in v0.4, only that format leaves the release.
- Heavy formats are never offered as optional downloads: only language data may be downloaded ([ADR 0001](docs/adr/0001-files-never-leave-the-device.md)).

## v0.2 Foundation

**Goal:** Rantakit looks and behaves the way RANTAKIT.md describes, on desktop, in Indonesian and English, for the Tools that already exist: Kompres Foto, Ubah Format Foto, Ubah Ukuran Foto, and Kompres PDF (compress photo, convert photo, resize photo, compress PDF), which v0.1.0 combined in one screen.

**Stories:** [docs/stories/v0.2.md](docs/stories/v0.2.md)

**Epics:**

- **Engine foundation** (first, before the other epics): split `rantakit_core` into `rantakit-image` and `rantakit-pdf`, create `rantakit-tasks`, move Result naming and History out of `src-tauri`, and give every engine progress, cancel, and error kinds ([ARCHITECTURE.md](ARCHITECTURE.md)). Add a CI workflow that runs the Rust tests and the frontend type check on every push and pull request.
- **New look:** clean and minimalist, teal on white, Fredoka and Plus Jakarta Sans, Keyline Icons (rounded), light and dark themes with the dark palette decided.
- **Home screen:** "Terakhir dipakai" (Recent tools) above "Semua peralatan" (All tools), All tools grouped by Job, and Tool search.
- **Coming soon cards:** every Tool planned before 1.0 that is not built yet appears in All tools and in search, greyed out with a "Segera hadir" (Coming soon) badge. It cannot be tapped, a screen reader reads it as "<Tool name>, segera hadir", it follows the platform (Scan Dokumen only on Android), and it disappears when the Tool ships.
- **Three-step Tool layout:** Files, Options, Result, with desktop steps 1 and 2 side by side and Open / Show in folder on the Result (plus Share on Windows).
- **Compression by outcome:** *Sesuaikan batas ukuran* (Fit a size limit, the default) and *Kecilkan saja* (Just make it smaller), with *Paling kecil / Seimbang / Paling jernih* (Smallest / Balanced / Clearest); the 1–100 quality number moves into Advanced options.
- **Size limits and Presets:** the last 5 Size limits as quick choices, and user-named Presets.
- **New Result names:** `ijazah (compressed).pdf` and the other suffixes in RANTAKIT.md, replacing `name-rantakit.ext`.
- **Riwayat (History):** replaces the "Terbaru" (Latest) page. Keeps 30 days, with **Hapus riwayat** (Clear history) and the **Simpan riwayat** (Keep history) switch. It starts empty: v0.1.0 History is not carried over.
- **Settings:** a dialog inside the main window, replacing the separate Settings window.
- **English:** every string translated, with "Ikuti sistem" (Follow system), Bahasa Indonesia, and English in Settings.

**Done when:**

- CI passes on Windows and Linux, and the product rules (Result names, the Original untouched, History) are tested in `rantakit-tasks`.
- All strings exist in Indonesian and English, and no jargon from the GLOSSARY.md list appears outside Advanced options.
- Text and buttons meet WCAG AA contrast in light and dark themes.

**Open decisions:** none. The light and dark color values come from the design board (a Claude Artifact) and are recorded in story N1.

## v0.3 Android

**Goal:** the same Tools as v0.2 work on a low-end Android phone, and people can reach them straight from WhatsApp, Gallery, or Files.

**Stories:** [docs/stories/v0.3.md](docs/stories/v0.3.md)

**Epics:**

- **Android build** of the existing Tools: Kompres Foto, Kompres PDF, convert photo, resize photo, as two signed APKs (arm64 and armv7) released with the desktop files from one tag.
- **Phone layout:** one column, actions at the bottom, bottom navigation, Android back gesture; the Result as its own screen; Settings as a full page.
- **Share → Rantakit:** files shared into Rantakit open with only the Tools that fit them.
- **Results in `Download/Rantakit/`**, visible in the Files and Gallery apps, and shared through the system share sheet.
- **Tasks in the background:** every Task runs in a foreground service with a progress notification, so switching apps doesn't stop it.
- **History ("Riwayat") on Android.**
- **"Versi baru" (New version)** ([RANTAKIT.md §7](RANTAKIT.md#distribution-and-updates)): installs updates on Windows and the Linux AppImage; the `.deb`, `.rpm`, and the APK from GitHub open the download page.

**Done when:**

- Compressing a 10-page scanned PDF takes under 5 seconds on an Android 8.0 emulator with 3 GB of RAM, recorded as an estimate; the check on a real phone runs on the v0.8 beta.
- Works on Android 8.0 and later.
- Both APKs are published on GitHub Releases, signed with the release keystore.

**Open decisions:** none; settled while writing the stories (2026-11-03).

## v0.4 More Tools

**Goal:** people can put PDFs together the way a portal asks for them (Job 3), turn paper documents into one clean PDF (Job 2), share ID copies and photos without them being misused (Job 5), and convert and compress the other formats they receive (Job 1).

**Stories:** [docs/stories/v0.4.md](docs/stories/v0.4.md)

**Epics:**

- **Shared pieces** (first, before the other epics): PDF page thumbnails rendered with `hayro`, starting by measuring `hayro` without its embedded CJK character maps and checking real Indonesian PDFs; files in a chosen order; several Results from one file.
- **Arrange PDF (Job 3):** Gabung PDF (merge); Pisah PDF (split, and extract pages); Atur Halaman PDF (arrange pages: reorder, rotate, delete).
- **Paper → PDF (Job 2):**
  - **JPG ke PDF** (JPG to PDF) with crop and straighten, on all platforms.
  - **Scan Dokumen** (scan document) on Android: camera, automatic edge detection, crop, straighten, clean up, several pages into one PDF.
- **Share safely (Job 5):** Beri Watermark (add a watermark to photos and PDFs); Hapus Info Lokasi (remove location and camera info from photos).
- **More formats (Job 1):**
  - **More photo conversion:** HEIC and WebP → JPG or PNG.
  - **PDF ke JPG** (PDF to JPG), rendered with `hayro`.
  - **Compress office documents:** Word, Excel, and PowerPoint files.

**Done when:**

- Each Tool follows the three-step layout and its Result names, on all platforms.
- "Turn these 3 photos of your KK into one PDF" works unaided on desktop and Android.
- "Put 'Hanya untuk verifikasi Bank X' (Only for Bank X verification) on this KTP (ID card) photo" works unaided.
- Each new format is within the size budget. A format that does not fit leaves v0.4 by the size gate instead of holding the release back.

**Open decisions:** Advanced options for each Tool; how clean-up works on low-end phones; the page rendering fallback if `hayro` fails real documents (Android's `PdfRenderer` plus a fix to `hayro`); whether Kompres Dokumen accepts the older `.doc`, `.xls`, and `.ppt`. The UI and UX were settled while writing the stories (2026-11-07).

## v0.5 Audio

**Goal:** people can make audio files (for example WhatsApp voice notes) smaller and convert them to common formats.

**Epics:** compress audio; convert audio.

**Done when:** within the size budget on all platforms.

**Open decisions:** supported formats; Tool names; Advanced options.

## v0.6 Video

**Goal:** people can make videos small enough to send or upload.

**Epics:** compress video, with Size limit mode. The epic starts with a prototype that proves the native encoders work: MediaCodec through the NDK on Android, Media Foundation on Windows, and a system encoder on Linux, including inside Flatpak ([ADR 0008](docs/adr/0008-h264-through-native-encoders-only.md)).

**Done when:** within the size budget, and usable on a ~3 GB RAM Android phone.

**Open decisions:** supported formats; Advanced options; speed bar on low-end phones; what happens if Linux has no usable encoder (ADR 0008 moves video to after 1.0).

## v0.7 Word/Excel to PDF

**Goal:** people can turn Word and Excel files into PDFs that portals accept.

**Epics:**

- **Fit the size budget first.** `office2pdf` adds about 38 MB to the Android arm64 APK before fonts ([ADR 0009](docs/adr/0009-size-budget.md)). Slim it (unused Typst parts, subset Carlito and Liberation fonts, anything a release build can drop) and measure again. If it still does not fit, decide between raising the caps and moving the Tool to after 1.0.
- **Word ke PDF and Excel ke PDF** (Word to PDF, Excel to PDF), labeled best effort.

**Done when:** within the size budget, and common documents (letters, CVs, simple tables) look the same as in the original app.

**Open decisions:** which layouts and fonts must be supported; Advanced options.

## v0.8 Ready for 1.0

**Goal:** everything planned for 1.0 is stable, accessible, and small.

**Epics:** bug fixes; accessibility pass (screen reader, keyboard, tap targets); app size pass on every platform; a beta release on Windows, Linux, and Android for the usability test.

**Done when:**

- The beta passes the usability test with 5 people on every core scenario, on desktop and Android.
- The app makes no network calls besides the update check and language downloads.
- The Android build has 0 trackers in an Exodus Privacy report.

**Open decisions:** none yet.

## 1.0

The catalogue of v0.2 to v0.8 works on Windows, Android, and Linux, minus any format the size gate moved out.

## After 1.0

In order, without versions yet:

1. **Get text:** Ambil Teks dari Gambar/PDF (get text from an image or PDF), with Indonesian and English included in the app.
2. **Protect PDF:** password-protect PDF, redaction, a simple drawn signature.

## Not in this roadmap

- **Distribution channels** (SignPath, Microsoft Store, Google Play, F-Droid, Flathub): decided later.
- **Where donations are mentioned** ([ADR 0002](docs/adr/0002-free-forever-no-feature-gating.md)): not scheduled.
