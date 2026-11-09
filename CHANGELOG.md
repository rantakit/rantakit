# Changelog

All notable changes to Rantakit are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- On Windows and Linux, a hamburger menu at the start of the title bar opens File, Edit, View, Go, and Help. From it you can open files, open Settings, undo in Arrange PDF Pages, cut, copy, and paste in text fields, change the text size and theme, go between pages, and report a problem. Each item shows its shortcut, if it has one
- Report a problem in Settings › About opens the project's GitHub Issues

### Changed

- Ctrl+O (Cmd+O on macOS) opens Open… to choose files for any Tool, instead of starting Compress Photo. Ctrl+Shift+O no longer starts Compress PDF
- Source code and New version link to the project's new home, github.com/rantakit/rantakit
- The Fit a size limit option is now labelled just Fit
- Home is new. On desktop a card takes dropped files or opens Choose files; on Android a round add button offers Choose files and Scan Document. Under it, seven Tools as icons, the ones used lately first, with All at the end, then Latest results: the ten latest Tasks, each opening its Result, with See all leading to History
- Every Tool, grouped and described as Home showed them before, moved to Tools, a new item between Home and History. Ctrl+2 opens it, and History moves to Ctrl+3
- Convert to JPG (photos, or the pages of a PDF), Convert to PNG, and Convert to PDF (photos into one PDF) replace Convert Photo, PDF to JPG, and JPG to PDF. Their old names still find them in search, and Presets, History, and recently used Tools carry over. Word to PDF and Excel to PDF are no longer listed as coming soon; Convert to PDF says they are
- Open… and Home's Choose files also take Word, Excel, and PowerPoint files
- History has an inbox icon
- The window opens larger, at 1280×800. On Windows and Linux it shrinks to fit smaller screens, such as a 1366×768 laptop, and never goes below its minimum size

### Fixed

- The KB/MB choice next to the size limit field is as tall as the number field beside it

## [0.4.0] - 2026-11-08

Rantakit 0.4.0 adds ten Tools, on Windows, Linux, and Android, for putting PDFs together the way a portal asks, turning paper into one PDF, sharing ID copies safely, and handling more formats. Rantakit is in early development and is not stable yet.

### Added

- "Gabung PDF" (Merge PDF) joins several PDFs, in the order you set, into `merged <date>.pdf`. A locked or damaged PDF stops the Task before anything is written and is named in the Result
- "Pisah PDF" (Split PDF) takes chosen pages, such as `1-3, 5`, into a new PDF, or splits every page, or every few pages, into separate PDFs. With one PDF, page thumbnails sit under the page field and stay in step with it
- "Atur Halaman PDF" (Arrange PDF Pages) shows every page of a PDF: tick pages to turn or delete them, drag them (or press Alt and an arrow key) to change the order, undo with "Urungkan" or Ctrl+Z, and save it as `(edited).pdf`. Leaving with unsaved changes asks first
- "JPG ke PDF" (JPG to PDF) turns photos into one PDF on A4, F4, or pages the shape of each photo. Rantakit finds the paper in each photo and offers to crop and straighten it, with a magnifier for touch; a photo that needs no change is put in as it is
- "Scan Dokumen" (Scan Document) on Android: take photos of paper with the camera, or add some from the Gallery, crop and straighten them, and save them as one PDF. "Bersihkan" (Clean up) takes out the paper's shade and shadows and keeps the colors. Rantakit asks for the camera at the first scan, after saying why
- "Beri Watermark" (Add Watermark) puts text, such as "Hanya untuk verifikasi Bank X" (Only for Bank X verification), and optionally today's date, on photos and PDFs: repeated across, in the middle, or in a corner, with a live preview. On a PDF it is real text over each page
- "Hapus Info Lokasi" (Remove Location Info) takes location, camera, and other photo info out of JPG and PNG photos without touching the picture. Photos with nothing to remove are left as they are
- HEIC and WebP photos, such as an iPhone's, open in every photo Tool. Their Results are JPG unless PNG is chosen
- "PDF ke JPG" (PDF to JPG) turns pages of one or several PDFs into JPG photos, at "Standar" (Standard, 150 dpi), "Tajam" (Sharp, 300 dpi), or a resolution from Advanced options
- "Kompres Dokumen" (Compress Document) makes Word, Excel, and PowerPoint files smaller by saving their pictures again, with the same Levels and Size limit as "Kompres PDF". `.docx`, `.xlsx`, `.pptx`, and their macro versions are accepted; the older `.doc`, `.xls`, and `.ppt` are refused, with how to save them in the new format
- When one file becomes several Results, they go into a new folder named after it, such as `ijazah (pages)/`. The Result lists them with "Buka folder" (Open folder) and "Bagikan semua" (Share all), and History keeps the Task as one entry
- Tools that join files, such as "Gabung PDF" and "JPG ke PDF", have an ordered file list with drag handles and a menu to move a file up or down; each move is announced to screen readers
- Files chosen or dropped on Home show the Tools that fit them, recently used first, and search knows more words for each Tool
- Office files and HEIC and WebP photos can be dropped on Home and shared to Rantakit from other Android apps

### Changed

- Every photo Result keeps the Original's color profile, such as an iPhone's Display P3, so colors don't fade after compressing or converting
- The interface calls Tools "peralatan" instead of "alat": "Semua peralatan" (All tools), "Cari peralatan" (Search tools)

## [0.3.0] - 2026-11-07

Rantakit 0.3.0 adds Android, with the same Tools as Windows and Linux, and opens files shared from other apps. Rantakit is in early development and is not stable yet.

### Added

- "Versi baru" (New version): at launch Rantakit checks GitHub for a newer version and shows a quiet button in the title bar. On Windows and the Linux AppImage one click installs it; the .deb and .rpm open the download page. It can be turned off in Settings, under "Tentang" (About)
- Android: photos and PDFs are chosen with the system file picker, keep their real names, are read where they are, and every Result is saved as a new file in `Download/Rantakit/`, where Gallery and Files find it. Android 8 and 9 ask for the storage permission at the first Task, after saying why
- Android APKs on every release, signed with one release key so each new version installs over the last and keeps History, Presets, and Settings: `Rantakit_<version>_arm64.apk` for almost every phone, and `Rantakit_<version>_armv7.apk` for older ones
- Android phone layout: the Tool page is one column with the main button at the bottom, and the Result is its own screen, where "Buka" (Open) asks which app to open it in and "Bagikan" (Share) opens the share sheet. Settings is a page listing its parts, without "File hasil" (Result files), since Results always go to `Download/Rantakit/`
- Android back gesture: it closes an open dialog first, then goes back one step as Back does on desktop, asking first before files would be lost; on Home it closes the app
- Android tablets and phones turned sideways use the wide layout from 768 px, and turning the phone keeps the files, options, and a running Task
- "Bagikan semua" (Share all) above a list of Results, on Windows and Android
- Android Tasks keep going when you switch apps, with a notification such as "Kompres Foto · 3 dari 12 file" (Compress Photo · 3 of 12 files). If Rantakit is out of sight when the Task ends, "Selesai" (Done) says how many Results are in `Download/Rantakit/` and how many files failed, and tapping it shows the Result. Android 13 and later ask once, at the first Task, whether Rantakit may show notifications, after saying why
- Android share sheet: photos and PDFs shared from WhatsApp, Gallery, Files, or any other app open in Rantakit, which shows the Tools that fit them or why none does. Rantakit can be closed, in the background, or open; while a Task runs it asks you to wait for it first
- History on Android and Windows has "Bagikan" (Share) for a Result and "Bagikan semua" (Share all) for a Task with several, next to "Buka". On a phone each entry's buttons sit under its file names, and a Result moved or deleted in another app is marked as missing as soon as you come back to Rantakit
- "Versi baru" on Android: the APK from GitHub checks for a newer release at launch and shows a button at the top of Home. Its dialog says to download the new APK and install it, keeping History and settings, and opens the release page. "Cek versi baru" in Settings, under "Tentang", turns it off

### Changed

- "Kompres PDF" (Compress PDF) works on several pages at once and tries fewer qualities for a Size limit. A 10-page scan takes about 3 seconds instead of 7 on an Android 8 emulator, and about 7 seconds instead of 39 with a 1 MB limit, using about 150 MB more memory while it runs
- Only release candidates, such as `v0.3.0-rc.1`, are marked as pre-releases on GitHub, so the "Versi baru" check finds every final version

## [0.2.0] - 2026-11-03

Rantakit 0.2.0 for Windows and Linux: everything in 0.2.0-rc.1 and 0.2.0-rc.2, plus the changes below. Rantakit is in early development and is not stable yet.

### Changed

- Files chosen with "Buka" (Open) that no Tool can take now show why, with "Pilih file lain" (Choose other files) to pick again

## [0.2.0-rc.2] - 2026-11-02

Second release candidate for 0.2.0, for Windows and Linux, with a new window layout. Rantakit is in early development and is not stable yet.

### Added

- Rantakit's own title bar on Windows and Linux, with Back, Forward, and a button to hide the sidebar
- Back and Forward move between Home, History, and Tool pages, with Alt+← and Alt+→ or the mouse's side buttons; Ctrl+B hides or shows the sidebar
- "Buka" (Open) in the sidebar: choose photos or PDFs, then pick a Tool for them
- The window opens in the middle of the screen at 1100 × 720, and can't be made smaller than 800 × 560

### Changed

- The sidebar and the page are two cards on the window background
- Tool pages fill the window: the file list and the options scroll on their own, and the main button always stays in view; on phones it stays above the tab bar
- A smaller Tool page header, and Esc on a Tool page goes Back
- Thin scrollbars in the app's colors

### Removed

- The "⋯" menu in the sidebar. The license and source code links are in Settings, under "Tentang" (About)

### Fixed

- Settings no longer flickers when switching tabs
- The window no longer bounces when a touchpad swipe goes past the edge
- Sidebar items, Tool cards, and other buttons show their hover color again

## [0.2.0-rc.1] - 2026-11-02

Release candidate for 0.2.0, for Windows and Linux, to run the usability test with five people before the final release. Rantakit is in early development and is not stable yet.

### Added

- A new look: Home lists every Tool grouped by what you want to do, with "Terakhir dipakai" (Recent tools), search, and "Segera hadir" (Coming soon) cards
- Drop files anywhere to see the Tools that fit them
- One Tool page for every Tool: files on the left, options on the right, then progress with Cancel, then the Result with sizes before and after
- "Kompres Foto" (Compress Photo) and "Kompres PDF" (Compress PDF) fit a Size limit such as "maks 200 KB", or "Kecilkan saja" (just make it smaller) in three plain choices
- "Ubah Format Foto" (Convert Photo) turns photos into JPG or PNG, and leaves photos already in that format unchanged
- "Ubah Ukuran Foto" (Resize Photo) makes photos Besar, Sedang, or Kecil (large, medium, small), and never enlarges them
- The last five Size limits as quick choices
- Presets: save a Tool's options under a name and pick them again later
- "Pilihan lanjutan" (Advanced options) for quality, image size, and dpi, each with a plain explanation
- "Bagikan" (Share) for Results on Windows
- Settings for where Results are saved: next to the original, or in one folder you choose
- Interface in English, following the system language, with a choice in Settings
- Text size choice, and bundled Fredoka and Plus Jakarta Sans fonts

### Changed

- "Riwayat" (History) lists one entry per Task, grouped by day, keeps 30 days, and can be turned off in Settings
- Settings opens as a dialog in the main window instead of a separate window
- Result names say what was done, such as `ijazah (compressed).pdf`, and never replace an existing file
- Error messages say what to do next
- Colors meet WCAG AA contrast in light and dark themes

### Removed

- History from 0.1.0 is cleared on first launch, because its format changed. Result files stay where they are

## [0.1.0] - 2026-10-30

First public release, for Windows and Linux. Rantakit is in early development and is not stable yet.

### Added

- Image compression for JPG and PNG, by quality (1 to 100) or by a target file size per image
- Conversion between JPG and PNG
- Resizing images to a maximum longest side (4K, Full HD, …)
- EXIF orientation is applied, so rotated phone photos stay upright
- PDF compression: re-encodes JPEG photos inside the PDF, optionally downscales them, and removes unused objects. Text and vector graphics are left untouched
- Batch processing with per-file progress and results
- Drag and drop files onto the window, or pick them from the file dialog
- Results are saved next to the original as `name-rantakit.ext`, and existing files are never overwritten
- History of recent jobs, with a shortcut to open the containing folder
- Settings for theme (light, dark, system) and UI zoom
- Interface in Indonesian

[Unreleased]: https://github.com/moez-rd/rantakit/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/moez-rd/rantakit/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/moez-rd/rantakit/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/moez-rd/rantakit/compare/v0.2.0-rc.2...v0.2.0
[0.2.0-rc.2]: https://github.com/moez-rd/rantakit/compare/v0.2.0-rc.1...v0.2.0-rc.2
[0.2.0-rc.1]: https://github.com/moez-rd/rantakit/compare/v0.1.0...v0.2.0-rc.1
[0.1.0]: https://github.com/moez-rd/rantakit/releases/tag/v0.1.0
