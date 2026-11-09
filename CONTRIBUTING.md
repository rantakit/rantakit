# Contributing to Rantakit

Thank you for helping Rantakit. Bug reports, ideas, and code are all welcome, from anyone.

Rantakit is in early development, and the current focus is 0.4.x maintenance: fixes, refactors, and small improvements. New Tools follow [ROADMAP.md](ROADMAP.md), so please open an issue before starting on a new Tool or a large change.

## Ways to help

- **Report a bug** with the [bug report form](https://github.com/rantakit/rantakit/issues/new?template=bug_report.yml).
- **Suggest a Tool or an improvement** with the [feature request form](https://github.com/rantakit/rantakit/issues/new?template=feature_request.yml). Tell us the everyday situation: what you were trying to get done, and where the file had to go.
- **Send a pull request** for an open issue, or after agreeing on the change in an issue.

**Never attach real personal documents** (ID cards, family cards, diplomas, bank statements, and the like) to an issue or pull request, yours or anyone else's. If a file triggers the bug, make one that contains nothing private, or describe how to make it.

## Where decisions live

Read the matching document before deciding something; most questions are already answered there.

| Question | Document |
| --- | --- |
| The rules for working in this repo | [AGENTS.md](AGENTS.md) |
| Product, UX, or the words in the UI | [RANTAKIT.md](RANTAKIT.md) |
| Whether work belongs in the current release | [ROADMAP.md](ROADMAP.md) |
| Which crate owns what, and which library does each job | [ARCHITECTURE.md](ARCHITECTURE.md) |
| What to call a concept in code, UI, or docs | [GLOSSARY.md](GLOSSARY.md) |
| Network use, pricing, feature availability, presets, dependencies, size | [docs/adr/](docs/adr/) |

Use the GLOSSARY.md terms everywhere: Tool, Task, Original, Result, Size limit, Preset, History.

## Project rules

Every change has to keep these. A pull request that breaks one is not merged, however useful it is.

- **Files stay on the device.** Rantakit makes no network calls besides the update check and the language downloads the user asks for. No telemetry, crash reporting, analytics, or cloud processing ([ADR 0001](docs/adr/0001-files-never-leave-the-device.md)).
- **The Original stays untouched.** Every Result is written to a new file; the Original is never modified, moved, or overwritten.
- **UI text is plain, warm Indonesian** in GLOSSARY.md wording, with English following the same tone. The words in its "Words never shown by default" list (quality percentages, DPI, EXIF, OCR, and so on) appear only inside Advanced options. Every string goes in both `src/i18n/id.ts` and `src/i18n/en.ts`.
- **Crates under `crates/` stay free of Tauri**, so the same engines and product rules serve desktop and Android ([ADR 0004](docs/adr/0004-engines-and-tasks-in-tauri-free-crates.md)).
- **New dependencies build from source** for Windows, Linux, and the Android NDK, with licenses compatible with AGPL-3.0-or-later. No prebuilt binaries or proprietary code ([ADR 0005](docs/adr/0005-dependency-policy.md)). Mind the app size, too ([ADR 0009](docs/adr/0009-size-budget.md)).
- **Docs are written in English.** Indonesian Tool names and UI text are quoted, with an English translation in brackets where they first appear.

## Setting up

You need:

- **Rust** (stable), from [rustup.rs](https://rustup.rs). On Windows, use the default MSVC toolchain.
- **[Bun](https://bun.sh)** for the frontend dependencies and scripts.
- **The Tauri 2 prerequisites** for your platform, from [tauri.app/start/prerequisites](https://tauri.app/start/prerequisites/):
  - **Windows:** Microsoft C++ Build Tools with "Desktop development with C++", and Microsoft Edge WebView2, which Windows 10 and 11 already have.
  - **Linux:** a C compiler and the WebKitGTK development packages. On Debian and Ubuntu, CI installs these:

    ```bash
    sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libssl-dev
    ```

    The Tauri page lists the packages for Fedora, Arch, openSUSE, and others.
  - **Android:** see [Android](#android) below.

Then install the frontend dependencies:

```bash
bun install
```

### Android

Android builds need Android Studio with an SDK, the NDK (Side by side), and the SDK Command-line Tools, plus the Rust targets:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android
```

Set `JAVA_HOME` (Android Studio's `jbr`), `ANDROID_HOME`, and `NDK_HOME`. On Windows, turn on Developer Mode, since the build links the Rust library into the Android project. tauri-cli 2.12 can't tell how `bun run` started it, so call the CLI directly and name bun:

```bash
npm_execpath="$(which bun)" bun node_modules/@tauri-apps/cli/tauri.js android build --debug --apk --target x86_64
```

The emulator, and any phone, needs Android System WebView 111 or later. The Tauri prerequisites page has more on the Android setup.

## Running

```bash
bun run tauri dev       # the desktop app, reloading as you edit
bun run tauri build     # a production bundle
```

## Checks

Run these before you open a pull request. CI runs the type check and the Rust tests on Windows and Linux, and checks that the Android app still compiles.

```bash
bun run typecheck                        # type-check the frontend
bun run build                            # type-check and build the frontend into dist/
cargo test --workspace                   # Rust tests
cargo clippy --workspace --all-targets   # Rust lints
cargo fmt --all                          # format the Rust code
```

- `src-tauri` embeds the built frontend, so run `bun run build` once before `cargo test --workspace` or `cargo clippy`; otherwise the app crate doesn't compile.
- Not all of the code is formatted with `cargo fmt` or free of Clippy warnings yet. Add no new warnings, and if `cargo fmt --all` changes files you didn't touch, leave those changes out of your pull request.
- Engine and Task changes need Rust tests. Test fixtures committed to the repo are generated (like `crates/rantakit-image/examples/gen_fixtures.rs`) or openly licensed; real documents are tested only from a local folder listed in `.gitignore`, never committed (see [ARCHITECTURE.md](ARCHITECTURE.md#tests-and-ci)).

## Commits

Commits follow [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/): `feat:`, `fix:`, `docs:`, `refactor:`, `perf:`, `chore:`, and so on, followed by a short sentence in English, such as:

```text
fix: keep the color profile when compressing a PNG
feat: add a hamburger menu to the title bar on Windows and Linux
```

## Changelog

Every change users can notice gets a line in [CHANGELOG.md](CHANGELOG.md), under `## [Unreleased]`, in the `### Added`, `### Changed`, `### Fixed`, or `### Removed` section ([Keep a Changelog](https://keepachangelog.com/en/1.1.0/)). Write it for users: what they can now do or what changed for them, in the GLOSSARY.md words, with UI text quoted as the app shows it. Refactors and other changes users can't notice need no line.

## Pull requests

1. Fork the repository, or create a branch if you have access. Name it after the kind of change, such as `fix/pdf-rotate-crash` or `feat/resize-photo-presets`.
2. Make your change, with the checks above passing and CHANGELOG.md updated.
3. Open a pull request into `main` and fill in the template. Link the issue it closes, and add screenshots for UI changes, on a phone layout too if the change shows there.
4. A maintainer reviews it. Once approved and CI passes, it is merged into `main` with a merge commit, so keep your commits meaningful; there is no squash.

## License

Rantakit is licensed under the [GNU Affero General Public License v3.0 or later](LICENSE) (`AGPL-3.0-or-later`). By contributing, you agree that your contribution is licensed under the same license.
