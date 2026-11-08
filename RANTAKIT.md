# Rantakit product plan

> **All your file tools, no internet needed, free & open source.**

This is the product and design plan for Rantakit: who it is for, what it does, what it will never do, how it looks and behaves on each platform, and how we know it is working. It deliberately says nothing about code architecture.

Indonesian is the app's home language, so Tool names, UI text, and examples of what users type are quoted in Indonesian, with an English translation in brackets where they first appear.

Related documents:

- [GLOSSARY.md](GLOSSARY.md): the words we use (Tool, Task, Original, Result, Size limit, Preset, History, …)
- [ROADMAP.md](ROADMAP.md): the order in which the Tools and platforms below are delivered
- [ADR 0001](docs/adr/0001-files-never-leave-the-device.md): files never leave the device, and there is no telemetry
- [ADR 0002](docs/adr/0002-free-forever-no-feature-gating.md): free forever, with no feature gating
- [ADR 0003](docs/adr/0003-no-institution-presets.md): no built-in institution presets

## 1. Promises

These are the reasons Rantakit exists. Every decision below must keep them.

1. **Files never leave the device.** All processing is offline. No telemetry, no uploads, ever ([ADR 0001](docs/adr/0001-files-never-leave-the-device.md)).
2. **Free forever.** No subscription, paid tier, ads, or locked features ([ADR 0002](docs/adr/0002-free-forever-no-feature-gating.md)).
3. **Open source** under AGPL-3.0-or-later, so anyone can inspect and contribute.
4. **Friendly for everyone**, including people who are not comfortable with technology ("orang gaptek").
5. **Indonesia first.** Indonesian is the home language and Indonesian everyday needs come first; English is fully supported.

## 2. Users

| | Who | What they need from Rantakit |
|---|---|---|
| **Primary** | Everyday non-technical people: students, job and CPNS (civil service) applicants, office and admin staff, small business owners. Today they use iLovePDF, Smallpdf, or random "kompres PDF online" (compress PDF online) sites. | Get a file *accepted somewhere*, without understanding file formats. |
| **Secondary** | Privacy-sensitive professionals: legal, HR, healthcare, finance staff who handle confidential documents and are not allowed to upload them. | The same tools, with certainty that nothing leaves the device, and a little more control. Their needs set the quality bar. |
| **Not a target** | Power users and developers. | They already have command-line tools. Designing for them would make Rantakit worse for the primary user. |

## 3. Jobs to be done

The five Jobs users hire Rantakit for, in priority order:

1. **Make my file acceptable where I'm sending it.** A form or portal has a rule ("pas foto JPG, maks 200 KB" (ID photo, JPG, max 200 KB), "ijazah PDF, maks 1 MB" (diploma, PDF, max 1 MB)) or a chat app has a size limit.
2. **Turn paper into a clean PDF.** Photograph a KTP (ID card), KK (family card), ijazah (diploma), or receipt, crop and straighten it, save it as one PDF.
3. **Put documents together the way they're asked for.** Merge, split, extract, reorder, rotate, or delete pages.
4. **Get the text out** of a photo or a scanned PDF.
5. **Share it without it being misused.** Watermark an ID copy for one purpose ("Hanya untuk verifikasi Bank X, 31/10/2026" (Only for Bank X verification, 31/10/2026)), remove location and camera info from photos, and later protect or redact PDFs.

## 4. Tool catalogue

Tools are named with the words people search for (Kompres PDF, not invented names), and each has a one-line plain description underneath.

| Job | MVP | Later |
|---|---|---|
| 1. Make it acceptable | Kompres Foto and Kompres PDF (compress photo, compress PDF; both with Size limit mode), Ubah Format Foto (convert photo: JPG, PNG, HEIC, WebP → JPG or PNG), Ubah Ukuran Foto (resize photo), compress office documents (Word, Excel, PowerPoint), compress and convert audio, compress video, Word ke PDF (Word to PDF), Excel ke PDF (Excel to PDF) | |
| 2. Paper → PDF | **All platforms:** JPG ke PDF (JPG to PDF), with crop and straighten. **Android:** Scan Dokumen (scan document: camera, automatic edge detection, crop, straighten, clean up, multiple pages into one PDF) | |
| 3. Arrange documents | Gabung PDF (merge PDF), Pisah PDF (split PDF), Atur Halaman PDF (arrange PDF pages: reorder, rotate, delete, extract), PDF ke JPG (PDF to JPG) | |
| 4. Get text | | Ambil Teks dari Gambar/PDF (get text from an image or PDF), with Indonesian and English included in the app |
| 5. Share safely | Beri Watermark (add a watermark to photos and PDFs), Hapus Info Lokasi (remove location and camera info from photos) | Password-protect PDF, redaction, a simple drawn signature |

HEIC is in the MVP because users often receive iPhone photos through WhatsApp or Drive. The heavy formats (HEIC and WebP, office documents, audio, video, Word and Excel to PDF) are in the MVP only if they pass the size gate (see [App size](#app-size)). OCR (Job 4) comes in the first release after the MVP, because it carries the most risk: app size, accuracy for Indonesian, and speed on low-end phones.

## 5. Non-goals

Rantakit will not:

- Send files, file names, or usage data anywhere; have telemetry or crash reporting; or process files in the cloud
- Charge money, show ads, or lock any feature
- Edit PDF content (changing text or fonts inside a PDF)
- Edit images beyond crop, rotate, straighten, and clean up
- Edit video
- Offer cloud sync or accounts
- Provide certified or legal e-signatures
- Offer plugins or scripting for power users
- Ship presets for institutions such as CPNS (civil service recruitment) or LPDP (government scholarship) ([ADR 0003](docs/adr/0003-no-institution-presets.md))
- Support macOS or iOS until there is a funded Apple developer account

## 6. UX rules

### Design principles

Every Tool, including community-contributed ones, is reviewed against these:

1. **No jargon** (see below).
2. **Good defaults that just work.** One press of the main button gives a good Result with no options touched.
3. **Nothing is ever destroyed.** The Original is never modified, moved, or overwritten; every Result is a new file.
4. **Show the Result.** Size before and after, a visual preview, and obvious Open, Share, and Show in folder actions.
5. **Plain, warm Indonesian**, like a helpful friend rather than a government form. English follows the same tone.
6. **Errors say what to do next.** "This PDF is locked with a password. Enter it to continue.", not "Encrypted stream error".
7. **Accessible by default.** Large tap targets, readable text, screen reader and keyboard support, and color is never the only signal.

### No jargon

- **The rule:** a word is fine if it appears in the user's world, and jargon if it only exists inside file tools. JPG, PNG, PDF, KB, and MB are fine, because portals use them. Quality percentages, DPI, pixel dimensions, lossless, EXIF, metadata, and OCR are jargon. The full list lives in [GLOSSARY.md](GLOSSARY.md).
- **Tool names** use search vocabulary: "Kompres PDF", "Gabung PDF", "JPG ke PDF", "Scan Dokumen", "Ambil Teks dari Gambar", "Beri Watermark".
- **Compression is controlled by outcome, not by a quality slider:**
  - **Sesuaikan batas ukuran** (Fit a size limit, the default): the user types the Size limit from the portal and Rantakit finds the best quality that fits. If the limit can't be reached, Rantakit says so and offers the next step ("Can only reach 240 KB. Also shrink the image dimensions?").
  - **Kecilkan saja** (Just make it smaller): three choices, *Paling kecil* (Smallest), *Seimbang* (Balanced), *Paling jernih* (Clearest).
- **Advanced options** are collapsed in every Tool, every option has a safe default, and the app remembers whether the user left the section open.

### Size limits and Presets

- The last 5 Size limits the user typed are offered as quick choices.
- Users can save their own named Presets ("Pas foto CPNS: JPG, 200 KB" (CPNS ID photo: JPG, 200 KB)).
- Rantakit ships no institution presets.

### Ways into the app

- **Tool-first (home screen):** "Terakhir dipakai" (Recent tools) above "Semua peralatan" (All tools). All tools are grouped by Job (*Kecilkan & Ubah* (Shrink & Convert), *Scan*, *Atur PDF* (Arrange PDF), *Ambil Teks* (Get Text), *Bagikan dengan Aman* (Share Safely)), and a search box finds Tools by name ("kompres" (compress) finds Kompres PDF and Kompres Foto).
- **Coming soon:** MVP Tools that are not built yet appear in All tools and in search, greyed out with a "Segera hadir" (Coming soon) badge. They cannot be tapped and show no message, a screen reader reads them as "Kompres Video, segera hadir", they follow the platform (Scan Dokumen only on Android), and they disappear once the Tool ships. Tools in the Later column are never shown.
- **File-first:** drop files on the desktop window, or use **Share → Rantakit** on Android from WhatsApp, Gallery, or Files. Rantakit then shows only the Tools that fit those files. On Android this is expected to be the main way in.
- **History ("Riwayat"):** past Tasks and their Results, to reopen or share again.

### The three-step Tool layout

Every Tool follows the same layout:

1. **Files:** pick, drop, or share. Shows thumbnails, sizes, and page counts.
2. **Options:** defaults already filled in, Advanced options collapsed, one main button ("Kompres" (Compress), "Gabungkan" (Merge)).
3. **Result:** before and after (size, preview), then **Open / Share / Show in folder**, then "Proses file lain" (Process another file). Share appears only where the platform has a share sheet (Android and Windows, not Linux).

On desktop, steps 1 and 2 sit side by side on one screen. On a phone they are stacked, and the Result is its own screen.

### Where Results go and what they are called

- **Desktop:** next to the Original by default; the folder can be changed in Settings.
- **Android:** `Download/Rantakit/`, so Results show up in the Files and Gallery apps.
- **Names:** a short **English** suffix that says what happened, in every UI language:

| Tool | Result name |
|---|---|
| Kompres Foto / Kompres PDF / compress office documents | `ijazah (compressed).pdf` |
| Convert / resize photo | `foto (converted).jpg`, `foto (resized).jpg` |
| JPG ke PDF / Scan Dokumen | `scan 2026-10-31.pdf` |
| Gabung PDF | `merged 2026-10-31.pdf` |
| Pisah PDF / Atur Halaman PDF | `ijazah (pages 1-3).pdf`, `ijazah (page 1).pdf`, `ijazah (edited).pdf` |
| PDF ke JPG | `ijazah (page 1).jpg` |
| Beri Watermark | `ktp (watermarked).jpg` |
| Hapus Info Lokasi | `foto (cleaned).jpg` |
| Name already taken | `ijazah (compressed) (2).pdf` |

- **Several Results from one Original** (Pisah PDF splitting every page, PDF ke JPG) go into a new folder named after it, `ijazah (pages)/`, so they don't scatter around the Original. A taken folder name gets a number, like a file: `ijazah (pages) (2)/`.

### History

- On by default; keeps the last 30 days.
- **Hapus riwayat** (Clear history) clears it in one click. It never deletes Results, and the page says so plainly.
- **Simpan riwayat** (Keep history) in Settings turns History off completely, for shared computers and privacy-sensitive users.

### First launch and language

- No onboarding tour, no sign-up, no questions. The app opens on the home screen, and the first Tool is one tap away.
- The language follows the operating system. Settings offers "Ikuti sistem" (Follow system, the default), Bahasa Indonesia, and English.

## 7. Multiplatform design

### Platforms

| Tier | Platforms |
|---|---|
| **MVP** | Windows, Android, Linux |
| **Later** | macOS, then iOS (each needs a paid Apple developer account) |

**Lowest supported:** Windows 10+, Android 8.0+ with an up-to-date Android System WebView (Chrome 111 or later, updated through the Play Store), current Ubuntu LTS and Fedora. Rantakit must work well on a low-end Android phone with about 3 GB of RAM.

### One Rantakit, two layouts

- **Same catalogue everywhere.** The only differences come from hardware: Scan Dokumen (camera) is Android-only, and desktop covers the same Job with JPG ke PDF plus crop and straighten.
- **Same names, icons, words, and steps everywhere.** Someone who learned Kompres PDF on their phone recognizes it immediately on an internet-café PC.
- **Two layouts from one design system:**
  - **Phone:** one column, actions at the bottom, bottom navigation, system share sheet.
  - **Desktop:** sidebar navigation, drag and drop, several files visible side by side.
- **One Rantakit look, platform behavior.** The visual style is Rantakit's own on every platform, while behavior follows each platform: Android back gesture and share sheet, Windows title bar and keyboard shortcuts, the system file picker.

### Distribution and updates

| Platform | Channels |
|---|---|
| Windows | Installer signed through the SignPath Foundation (avoids the "Windows protected your PC" warning), published on GitHub Releases |
| Android | Google Play, F-Droid, APK on GitHub Releases |
| Linux | Flathub (primary); .deb, .rpm, and AppImage on GitHub Releases |

The in-app update check is on by default, shows a quiet "update available" notice with a one-click download, can be switched off in Settings, and is disabled where a store handles updates ([ADR 0001](docs/adr/0001-files-never-leave-the-device.md)). It ships in v0.3 for Windows and the Linux AppImage, which install the update themselves; the Linux .deb and .rpm, and the Android APK from GitHub, open the download page instead.

### App size

Keep the app as small as possible, because many users download it over prepaid mobile data. When a feature would make the app noticeably bigger, the lighter way to build it wins, even if it is slightly less clever. Indonesian and English language data are included; other languages are optional downloads the user asks for.

**The size gate.** The 1.0 app must fit these download sizes: 20 MB for the Android arm64 APK, 16 MB for the armv7 APK, 8 MB for the Windows installer, and 10 MB for the Linux `.deb` ([ADR 0009](docs/adr/0009-size-budget.md)). Every heavy format was measured against them; all fit except Word and Excel to PDF, which must be slimmed before it ships. A format over budget moves to after 1.0 until a lighter way is found, unless the caps are raised on purpose; it is never offered as an optional download ([ADR 0001](docs/adr/0001-files-never-leave-the-device.md)).

## 8. Design system

### Personality

**Calm, trustworthy, warm, local.** Closer to a neat, friendly service counter than a flashy online converter full of ads. The look is clean and minimalist: white space, thin lines instead of shadows, big clear buttons, no mascots, no decorative gradients. Reassurance ("File tetap di perangkatmu" (Your files stay on your device)) appears quietly where it matters.

### Color

- **Teal `#14B8A6`** is the brand color, taken from the icon. A darker teal is used for text and buttons wherever needed to reach WCAG AA contrast.
- **White `#FFFFFF`** is the background in light mode, with neutral greys for lines, the sidebar, and secondary text.
- **Red, amber, and green** are used only for status (error, warning, success).
- Themes: light, dark, and follow system.

### Typography

- **Fredoka** (the logo font): wordmark, page titles, and large headings only, such as "Kompres PDF" at the top of a Tool page.
- **Plus Jakarta Sans**: everything else, including Tool cards, buttons, options, file names, and numbers.
- Both fonts are bundled with the app, so nothing is downloaded.
- Base text size is **16 px**, larger than typical desktop apps, for older users and small, cheap screens.

### Icons

[Keyline Icons](https://keylineicons.com/) (MIT license), **rounded** variant, to match Fredoka and the warm personality.

| Style | Where |
|---|---|
| **Stroke** | Default everywhere: Tool cards, buttons, menus |
| **Fill** | Active or selected state on desktop and in controls (current sidebar item, selected option) |
| **Duotone / two-tone** | Active item in mobile bottom navigation |
| **Duotone, teal** | Tool icons on the home screen and at the top of a Tool page |

Every icon that stands for a Tool or navigation item has a **text label** next to it; icons are never used alone.

## 9. Success metrics

The app cannot measure anything itself ([ADR 0001](docs/adr/0001-files-never-leave-the-device.md)), so every metric comes from outside the app.

| Area | Metric | Bar |
|---|---|---|
| **North star** | Monthly active installs reported by the stores (Play Console), plus GitHub Release and Flathub downloads | No target yet; record a baseline after the MVP launch |
| **Quality** | Google Play rating | ≥ 4.5 |
| | Reviews that mention ease ("gampang"), offline use, or safety ("aman"), read manually each quarter | Tracked, no target |
| **Usability** | Test the beta before 1.0, then each major release after it, with 5 people from the primary audience (a parent, a CPNS applicant, a small-shop owner, …) on core scenarios: "Make this PDF under 1 MB", "Turn these 3 photos of your KK into one PDF", "Put 'Hanya untuk verifikasi Bank X' on this KTP photo" | At least 4 of 5 complete each scenario unaided in under 60 seconds |
| **Verified privacy** | Exodus Privacy report for the Android build | 0 trackers, no unexpected permissions |
| | Check on every release that the app makes no network calls besides the update check and language downloads | Passes on every release |
| **Performance** | Compress a 10-page scanned PDF on a low-end Android phone (~3 GB RAM) | Under 5 seconds |
| **Size** | Installed app size per platform | As small as possible; tracked every release |
| **Community** | First reply to new issues | Within 7 days (maintainer commitment) |

## 10. Open questions

- Where in the app donations are mentioned (ADR 0002 says one quiet place; the place is not decided yet).
- Distribution: when each channel opens, whether to also publish on the Microsoft Store, and whether 1.0 requires the stores.
- Whether Word and Excel to PDF can be slimmed to fit the size budget.
- Which additional OCR languages to offer as downloads first.

The dark-mode palette is decided in v0.2, and each Tool's Advanced options are decided at the start of its epic ([ROADMAP.md](ROADMAP.md)).

## 11. Changes from v0.1.0

v0.1.0 was experimental. Moving to this plan means:

- The main UI color changes from blue to teal, matching the icon.
- The "Terbaru" (Latest) page becomes **Riwayat** (History), and the home screen gains Recent tools and All tools.
- The 1–100 quality setting is replaced by Size limit and *Paling kecil / Seimbang / Paling jernih*, with the number moved to Advanced options.
- Result names change from `name-rantakit.ext` to descriptive English suffixes.
- English is added next to Indonesian, with a language choice in Settings.
