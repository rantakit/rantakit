# Rantakit architecture

This is the code map for Rantakit: which part owns what, the contract every engine follows, and which library or platform backend does each job. The product rules it serves live in [RANTAKIT.md](RANTAKIT.md), the order of work in [ROADMAP.md](ROADMAP.md), and the reasons behind the hard-to-reverse choices in the ADRs linked below.

Some of this is the target, not today's code. Today only `rantakit-core`, `rantakit-image`, `rantakit-pdf`, and `rantakit-tasks` exist, and the shell runs every Task through `rantakit-tasks`. The v0.2 "Engine foundation" epic moves the code to the layout below; other engine crates are created when their release starts.

## Parts

| Part | Folder | Owns |
|---|---|---|
| `rantakit-core` | `crates/rantakit-core` | The engine contract every engine shares: error kinds, progress, the cancel token, and writing a Result safely. No file-format code |
| `rantakit-image` | `crates/rantakit-image` | JPG and PNG, HEIC and WebP input, resize, watermark on photos, removing location and camera info |
| `rantakit-pdf` | `crates/rantakit-pdf` | Compress, merge, split, arrange pages, watermark on PDFs, render pages to JPG |
| `rantakit-office` | `crates/rantakit-office` | Compress Word, Excel, and PowerPoint files; Word and Excel to PDF |
| `rantakit-audio` | `crates/rantakit-audio` | Compress and convert audio |
| `rantakit-video` | `crates/rantakit-video` | Compress video |
| `rantakit-tasks` | `crates/rantakit-tasks` | The product rules shared by every platform (see below) |
| App shell | `src-tauri` | Turns frontend requests into Tasks; platform matters (file paths, share sheet, Result folder) |
| Frontend | `src` | One React app with a phone layout and a desktop layout; all words, icons, and error messages |

Every crate under `crates/` is free of Tauri ([ADR 0004](docs/adr/0004-engines-and-tasks-in-tauri-free-crates.md)). Crate names use kebab-case; Rust code imports them with underscores (`use rantakit_image::…`).

The app shell pulls in each engine crate through a cargo feature, so a platform build can leave a format family out. This is how the size gate in [RANTAKIT.md §7](RANTAKIT.md#app-size) is enforced.

## Engine contract

Every engine function follows the same shape, using the types in `rantakit-core`:

- **File in, file out.** It reads one or more local input files and writes its Result to a local output path chosen by `rantakit-tasks`, never over an existing file. Large inputs (video, audio, big PDFs) are streamed, never held whole in memory; image engines may load a whole photo internally.
- **Progress.** It reports progress through a callback, as a fraction that only goes up, or as "page *n* of *m*".
- **Cancel.** It checks a cancel token between steps and stops promptly. The Result is written to a temporary file and renamed only when complete, so a cancelled or failed file leaves no partial Result behind.
- **Error kinds, not messages.** It returns a typed error kind such as `PasswordProtected`, `Damaged`, `UnsupportedFormat`, `SizeLimitUnreachable { best }`, `NotEnoughSpace`, or `Cancelled`. The shell passes the kind to the frontend as a code, and the frontend turns it into an Indonesian or English message that says what to do next (RANTAKIT.md §6, principle 6).
- **File kinds from content.** Engines detect what a file is from its bytes, not its extension.

## `rantakit-tasks` and the shell

`rantakit-tasks` holds the rules that must behave the same on every platform:

- **Results:** names (`ijazah (compressed).pdf`, then `(2)` when the name is taken) and the rule that the Original is never modified, moved, or overwritten.
- **Tasks:** running a Tool over one or more files, with per-file progress and outcome.
- **History:** storage (one JSON file in the app data folder), 30-day retention, Hapus riwayat (Clear history), and the Simpan riwayat (Keep history) switch.
- **Size limits and Presets:** the last 5 Size limits and user-named Presets.
- **Tool catalogue facts:** each Tool's id, Job group, accepted file kinds, platforms, and whether it has shipped or is coming soon; and matching dropped or shared files to the Tools that fit them.

The shell stays thin. It deserializes requests, asks the platform where files are and where Results go, calls `rantakit-tasks`, and returns results and error codes. The frontend owns everything a user reads: Tool names and descriptions, icons, and messages, in both languages.

## Backends per format

A Tool keeps the same promise on every platform (same Tool, same output format, same quality bar), but its engine may use a different backend per platform ([ADR 0006](docs/adr/0006-per-platform-backends.md)). Every backend follows the dependency policy ([ADR 0005](docs/adr/0005-dependency-policy.md)).

| Format | Windows | Linux | Android |
|---|---|---|---|
| JPG and PNG, resize | `image` | `image` | `image` |
| PDF compress, merge, split, arrange | `lopdf` | `lopdf` | `lopdf` |
| HEIC input ([ADR 0007](docs/adr/0007-accept-hevc-patent-risk-for-heic-decoding.md)) | `heic` | `heic` | `heic` (the system decoder is the reserve if `heic` is too big) |
| WebP input (no WebP output) | `image-webp` | `image-webp` | `image-webp` |
| PDF to JPG | `hayro` | `hayro` | `hayro` (Android's `PdfRenderer` is the fallback if `hayro` fails real documents) |
| Compress office documents | `zip` plus `rantakit-image` | same | same |
| Audio in (.opus, .ogg, .m4a, .mp3, .wav) | `symphonia`, libopus for Opus | same | same |
| Audio out (MP3 only) | LAME | LAME | LAME |
| Video (H.264 in MP4 only, [ADR 0008](docs/adr/0008-h264-through-native-encoders-only.md)) | Media Foundation | System ffmpeg or GStreamer at runtime (to be proven by the v0.6 prototype) | MediaCodec and MediaMuxer through the NDK |
| Word and Excel to PDF | `office2pdf` with Carlito and Liberation fonts, labeled best effort; must be slimmed to fit the size budget | same | same |
| Finding the paper in a photo, straightening it (JPG ke PDF, Scan Dokumen) | `imageproc`, with draggable corners when detection misses | same | same |
| Photos into a PDF (JPG ke PDF) | `lopdf`, each JPG embedded as it is when it needs no change | same | same |

Native Android backends are called from Rust through the NDK (`ndk` crate), so engine logic stays inside the engine crate. Kotlin is used only for app plumbing.

## Android

- **Receiving shares:** a small Kotlin plugin of our own registers for the share sheet (one or several files) and hands `content://` URIs to Rust.
- **Reading:** Rust opens those URIs through `tauri-plugin-android-fs`, which returns an ordinary `std::fs::File`. Android does not let the app reopen that file through a path such as `/proc/self/fd/N`, so the Files step reads size, kind, and pages from the open file (`rantakit_tasks::file_info_of`), and a Task copies each Original into the app's cache when its turn comes, because the engines take paths. The copy is removed once its Result is saved.
- **Saving:** the engines write each Result into the cache; the shell then saves it as a new file in `Download/Rantakit/` through `tauri-plugin-android-fs` (MediaStore, which also makes photos show up in Gallery). The name follows `rantakit_tasks::result_name`, counted against the Results already there; MediaStore numbers any other taken name itself, so no file is replaced.
- **Permissions:** `WRITE_EXTERNAL_STORAGE` is declared with `maxSdkVersion="28"`, so only Android 8 and 9 ask for it.
- **Tasks in the background:** the Task itself runs in Rust; `TaskPlugin.kt` and `TaskService.kt` in the app module hold a foreground service for as long as it runs, so Android keeps the app alive when the user switches apps. `src-tauri/src/task_notice.rs` starts the service from the Task's own thread right after "Kompres" is pressed, while the app is on screen (Android 12 and later forbid starting one from the background), updates the progress notification at most twice a second, and stops it at the end. If the app is out of sight then, "Selesai" with the count of Results and failed files takes its place, and tapping either notification brings the app back as it was; coming back clears "Selesai". The words are in Rust (`notice_words.rs`), in the language the frontend sets with `set_language`. Kotlin commands run on the main thread, so Rust never waits for them from a command that may run there. If Android stops the app halfway, each Result being written is still pending in MediaStore, which hides it and removes it after a week (Android 10 and later), and the copies in the cache are removed at the next launch. Permissions:
  - `FOREGROUND_SERVICE`: to run a foreground service at all.
  - `FOREGROUND_SERVICE_MEDIA_PROCESSING`: the service type on Android 15 and later, which is meant for processing media files.
  - `FOREGROUND_SERVICE_DATA_SYNC`: the service type on Android 10 to 14, where processing local files falls under data sync.
  - `POST_NOTIFICATIONS`: asked once, at the first Task on Android 13 and later, after one line of reason; the Task runs even when it is refused, only without its notification.
- **WebView:** the frontend needs Chrome 111 or later (Tailwind v4 uses `color-mix()`, `oklch()`, and cascade layers). Android 8's factory WebView is Chrome 58 to 69, so a small old-JavaScript check in `index.html` shows how to update it from the Play Store instead of a blank screen.
- **Picking files:** photos and PDFs through the system file picker (`ACTION_OPEN_DOCUMENT`, through `tauri-plugin-android-fs`), which needs no permission. `tauri-plugin-dialog` is not used on Android because it opens the Photo Picker for photos, which hands over names like `33.jpg` instead of the real one.
- **Receiving shares:** `MainActivity` takes `SEND` and `SEND_MULTIPLE` for JPG, PNG, and PDF. `SharePlugin.kt` keeps the shared `content://` URIs from the launch intent and from each new intent until the frontend takes them (`take_shared_files`), so a share that starts the app is not lost before the page loads; while the app runs, it tells Rust through a channel, and `src-tauri/src/share_in.rs` emits "shared-files". The sending app's read grant lasts as long as Rantakit's task, which covers the Task that reads them. The frontend then goes the way of "Buka" on Home, after the leave question when a Tool page has files, and refuses the share while a Task runs. `match_files` reads the kind of `content://` files through their open file, as the Files step does.
- **Open and share:** "Buka" and "Bagikan", on the Result and in History, look up each Result's MediaStore URI (`scan_by_path`) and hand it to the system chooser or share sheet through `tauri-plugin-android-fs`. "Tampilkan di folder" is hidden, since Android has no file manager to show a file in.
- **New version:** the APK from GitHub can't use the updater plugin, which replaces the running app. At launch, unless "Cek versi baru" is off, `UpdatePlugin.kt` reads the version in the latest release's `latest.json` (the file the desktop updater reads) with `HttpURLConnection`. The request goes off the main thread and carries "Rantakit" as its user agent and nothing more. `apk_update.rs` compares that version with this one. A newer one shows "Versi baru" at the top of Home, whose dialog opens the release page; Rantakit never downloads or installs the APK, so it doesn't need `REQUEST_INSTALL_PACKAGES`. Builds for a store are made with `RANTAKIT_STORE_BUILD` set, which drops the check and its switch in Settings.
- **Missing Results in History:** Android 11 and later let the app see the files it saved in `Download/` by path, as do 8 and 9 with the storage permission, so History checks the path. Android 10 shows them only through MediaStore, so there a Result that isn't found by path is looked up with `scan_by_path` (`android::result_exists`). History checks again whenever Rantakit comes back on screen, since Results are usually moved or deleted in another app.
- **Back gesture:** `src/lib/backButton.ts` listens to `onBackButtonPress` only while a dialog or popover is open or a screen has a step to go back to, because listening takes over Android's own handling. An open overlay gets an Escape keydown, so it closes or asks exactly as Escape does; otherwise the newest screen's handler runs. With nothing to do, the listener is removed and Android closes the app itself, which avoids calling `exit` from the WebView.
- **Layout:** below 768 px (`useWideLayout`) the app shows the bottom tab bar, a one-column Tool page, and Settings as a page; from 768 px it shows the sidebar and the Settings dialog. The orientation is not locked, and the manifest's `configChanges` keep the WebView, and so every file and running Task, when the phone turns.
- **Update check:** `tauri-plugin-updater` is desktop only. The APK from GitHub reads `version` from the latest release's `latest.json` with a small HTTP client and only opens the download page, so it never needs `REQUEST_INSTALL_PACKAGES`. Store builds leave the check out.
- **Builds:** one APK per ABI, arm64-v8a and armeabi-v7a, signed with a release keystore kept in GitHub secrets (`ANDROID_KEYSTORE`, the keystore in base64; `ANDROID_KEYSTORE_PASSWORD`, also the key's password; `ANDROID_KEY_ALIAS`). The release workflow writes them to `src-tauri/gen/android/keystore.properties`, which `build.gradle.kts` signs with when it exists, and stops when a secret is missing, since an APK signed with another key can't install over the last one. Local release builds have no such file and come out unsigned. Pull requests run `cargo check` for both Android targets; full APKs are built only by the release workflow.
- **Speed:** `cargo run --release -p rantakit-tasks --example scan_speed` makes a 10-page scanned PDF and times Kompres PDF on it; its comment shows how to run it on an Android emulator. `rantakit-pdf` decodes and encodes up to 4 images at once on scoped threads, but shrinks only 2 at once, because the image crate's Lanczos3 holds a float copy of the page (about 90 MB at 300 dpi) while it works. A Size limit search stops when the qualities left are within 5 of each other and skips shrink steps that would shrink no image.

## Size experiment

The throwaway `spike/size` branch adds each heavy format as a `spike-*` cargo feature on v0.1.0, and its "Size experiment" workflow builds the baseline and each format alone for Windows (installer), Linux (`.deb` and AppImage), and Android (arm64-v8a and armeabi-v7a APKs), then writes a download-size table. The AppImage bundles its own WebKit, so `.deb` is the Linux number that shows what a format adds. The branch is kept so later epics can measure again; it is never merged.

- **Results and budget:** [ADR 0009](docs/adr/0009-size-budget.md). The 1.0 caps are 20 MB (Android arm64), 16 MB (Android armv7), 8 MB (Windows installer), and 10 MB (Linux `.deb`).
- **Over budget:** Word and Excel to PDF is the only format that does not fit yet; its epic slims it first. If formats still exceed the caps, they leave 1.0 in this order: Word and Excel to PDF, video, audio, PDF to JPG, office compression, HEIC.

## Tests and CI

- Each engine crate and `rantakit-tasks` has Rust tests against fixtures.
- A CI workflow runs the Rust tests and the frontend type check on every push and pull request, on Windows and Linux.
- Native backends (MediaCodec, Media Foundation) are checked by hand on a real low-end Android phone and a Windows PC before each release.
- **Fixtures** committed to the repo are generated (like `crates/rantakit-image/examples/gen_fixtures.rs`) or openly licensed. Real documents (ID cards, diplomas, CVs, bank statements) are tested only from a local folder listed in `.gitignore`, never committed.
