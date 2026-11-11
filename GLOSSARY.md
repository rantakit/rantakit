# Rantakit

Rantakit is a free, open source, offline toolkit that helps everyday people get their files into the shape they need: smaller, converted, scanned, arranged, or safe to share.

## Language

### Doing things to files

**Tool**:
One thing the app can do to files, such as Kompres PDF or Gabung PDF. Named with the words people search for. The UI calls Tools "peralatan", as in "Semua peralatan" and "Cari peralatan".
_Avoid_: feature, action, utility, converter, "alat"

**Task**:
One run of a Tool on one or more files. Every Task appears in History.
_Avoid_: job, operation, process

**Original**:
A file the user gives to a Tool. Rantakit never modifies, moves, or overwrites it.
_Avoid_: source, input

**Result**:
A new file produced by a Task, always saved separately from the Original.
_Avoid_: output, export

**Page**:
In Scan Dokumen, one photo as it goes into the PDF: cropped, straightened, turned, and in the chosen Look. The UI calls it "halaman", also while it is still being made ("Menyiapkan halaman…"). The photo itself is the Original.
_Avoid_: "foto" or "jepretan" for a page on the review screen, "scan" as a noun

**History**:
The list of past Tasks and their Results, shown in the UI as "Riwayat".
_Avoid_: recents, "Terbaru", log

**Recent tools**:
The Tools the user used most recently, shown first in Home's grid of seven Tools, which defaults for the platform fill up.
_Avoid_: "recent" on its own (ambiguous with History), favorites

**Latest results**:
The ten latest Tasks from History, shown on Home as "Hasil terbaru", with "Lihat semua" leading to History.
_Avoid_: "Terbaru" as a page name

**All tools**:
The page listing every Tool with its description, grouped by Job, shown in the navigation as "Peralatan" and titled "Semua peralatan".

### Options

**Size limit**:
The maximum file size a Result must meet, usually copied from a form or portal ("maks 200 KB").
_Avoid_: target size, quota, max size

**Preset**:
A saved, named set of Tool options created by the user, such as "Pas foto CPNS: JPG, 200 KB".
_Avoid_: profile, template, institution preset

**Look**:
How every Page of a Scan Dokumen looks: "Bersihkan" (background made white, writing made sharper) or "Warna asli" (as photographed). One choice for all Pages, labelled "Tampilan" for screen readers.
_Avoid_: filter, mode, enhance

**Advanced options**:
Options hidden by default inside a Tool. Every one has a safe default, so the user can ignore them.
_Avoid_: expert mode, settings (Settings is the app-wide page)

### Words never shown by default

These are jargon to the primary user. They may appear only inside Advanced options or developer documentation:

- quality percentage → use "Paling kecil / Seimbang / Paling jernih"
- DPI, resolution in pixels → use "ukuran gambar" with plain choices
- lossless, lossy, encode, codec, rasterize → do not mention
- EXIF, metadata → use "info lokasi dan kamera"
- OCR → use "ambil teks"

JPG, PNG, PDF, KB, and MB are **not** jargon, because forms and portals use them. JPG and JPEG are always treated as the same format.

## Product vocabulary (not shown in the UI)

**Job**:
One of the five jobs-to-be-done a user hires Rantakit for: make a file acceptable, turn paper into a PDF, arrange documents, get the text out, share safely. Used only in planning and design discussions.
_Avoid_: using "job" for a Task
