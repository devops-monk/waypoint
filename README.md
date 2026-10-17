# Waypoint

An honest, private job search copilot for macOS, Windows and Linux. It finds real jobs on company job boards, scores how well you fit using only facts from your profile, and tracks every application. Your data stays on your computer, and AI calls use your own key.

## What works today (Phase 0 + early Phase 1)

- **Companies:** follow companies by pasting a careers-page link (Greenhouse, Lever, Ashby) or choosing from 110 suggestions. You can also watch Remotive for remote roles by keyword. Boards that fail or go empty are flagged.
- **Discover:** an inbox of new roles ranked by fit, with keyboard triage (J/K to move, S to save, X to skip), a remote filter, search, and a warning on old postings.
- **Fit scoring:** gates for eligibility, language and location, then four weighted dimensions (skills, experience, culture, career). Also covers strengths and gaps, ghost-job signals, and the keywords a posting screens for. The model scores each dimension and the weighted total is computed in code, so the number is reproducible.
- **Pipeline:** a drag-and-drop board from Saved to Hired, with closed roles kept to the side.
- **Profile:** import a PDF, .txt or .md resume, or paste the text. The AI drafts your profile, and you review it before saving.
- **Settings:** bring your own key for Claude, OpenAI or OpenRouter, or run free and fully local with Ollama. Keys live in the OS keychain. A usage panel shows token spend.
- **Everywhere:** ⌘K command palette, ⌘N to add any job by hand (for example from LinkedIn), ⌘1–5 to switch views, light and dark mode, and native window translucency on macOS and Windows.

## Layout

```
crates/core      domain model, SQLite storage + migrations, dedup keys, scoring
crates/sources   job sources: Greenhouse, Lever (US/EU), Ashby, Remotive; SSRF-guarded HTTP
crates/ai        Anthropic + OpenAI-compatible adapters, structured-output prompts
apps/desktop     Tauri 2 shell (src-tauri/) and React 19 UI (src/)
```

## Develop

Requirements: Rust 1.85+, Node 20+, pnpm, and the Tauri CLI (`cargo install tauri-cli`).

```sh
cd apps/desktop
pnpm install
pnpm tauri dev          # run the app
cargo test --workspace  # from the repo root: Rust unit tests
pnpm typecheck          # frontend types
```

Check a live job source without the UI:

```sh
cargo run -p waypoint-sources --example fetch -- ashby supabase
```

Build installers for the current OS: `pnpm tauri build` produces a .dmg/.app on macOS, .msi/.exe on Windows, and .deb/.rpm/AppImage on Linux.

## Principles

- **Never invent facts.** Scores and generated text use only what's in your profile.
- **Never auto-submit.** Waypoint prepares and tracks; you apply.
- **Public data only.** It reads job boards companies publish for embedding. It does not log in to or scrape LinkedIn or Indeed.
- **Local first.** The SQLite database lives in your app data folder and there's no Waypoint server.

See `NOTICE.md` for the open-source work this builds on.
