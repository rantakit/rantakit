# Rantakit

Rantakit is a free, open source, offline file toolkit for everyday Indonesian users: a Tauri 2 app with a React frontend in `src/`, the app shell in `src-tauri/`, the file engines in `crates/` (`rantakit-image`, `rantakit-pdf`, `rantakit-office`, and their shared contract `rantakit-core`), and the product rules in `crates/rantakit-tasks/`.

## Read before deciding

- **Product, UX, or UI wording:** [RANTAKIT.md](RANTAKIT.md)
- **Scope**, whether work belongs in the current release: [ROADMAP.md](ROADMAP.md)
- **Code structure**, which crate owns what, or choosing a library: [ARCHITECTURE.md](ARCHITECTURE.md)
- **Naming** a concept in code, UI, or docs: [GLOSSARY.md](GLOSSARY.md). Use its terms (Tool, Task, Original, Result, Size limit, Preset, History).
- **Network use, pricing or feature availability, or presets:** the matching record in [docs/adr/](docs/adr/)

## Rules

- **Files stay on the device.** The only network calls are the update check and language downloads the user asks for ([ADR 0001](docs/adr/0001-files-never-leave-the-device.md)).
- **The Original stays untouched.** Every Result is written to a new file; the Original is never modified, moved, or overwritten.
- **UI text** is plain, warm Indonesian in GLOSSARY.md wording. Words in its "Words never shown by default" list appear only inside Advanced options.
- **Crates under `crates/` stay free of Tauri**, so the same engines and product rules serve desktop and Android ([ADR 0004](docs/adr/0004-engines-and-tasks-in-tauri-free-crates.md)).
- **New dependencies build from source** for Windows, Linux, and the Android NDK, with AGPL-compatible licenses ([ADR 0005](docs/adr/0005-dependency-policy.md)).
- **Docs are written in English.** Indonesian Tool names and UI text are quoted, with an English translation in brackets where they first appear.

## Working

- Use bun for frontend dependencies and scripts; CI installs with `bun install --frozen-lockfile`.
- Commits follow Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`).
