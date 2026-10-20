# Waypoint

An honest, private job search copilot for macOS, Windows and Linux. It finds real jobs on company job boards, scores how well you fit using only facts from your profile, and tracks every application. Your data stays on your computer, and AI calls use your own key.

## What works today

- **Companies:** follow a company by pasting its careers-page link. Waypoint reads Greenhouse, Lever, Ashby, Workday, SmartRecruiters, Workable, Recruitee, Personio and Teamtailor boards, and offers 129 suggestions to start from. Boards that fail or go empty are flagged. For Workday and SmartRecruiters, which list jobs without descriptions, the description is fetched the first time you open a job.
- **Job searches:** watch a keyword search on remote boards (Remotive, Himalayas, Remote OK), Hacker News' monthly "Who is hiring?" thread, national job boards (Arbeitsagentur in Germany, Platsbanken in Sweden, Jobnet in Denmark), or Adzuna for the US, UK, India and most of Europe (needs your own free Adzuna key, kept in the keychain).
- **Discover:** an inbox of new roles ranked by fit, with keyboard triage (J/K to move, S to save, X to skip), a remote filter, search, and a warning on old postings.
- **Fit scoring:** gates for eligibility, language and location, then four weighted dimensions (skills, experience, culture, career). Also covers strengths and gaps, ghost-job signals, and the keywords a posting screens for. The model scores each dimension and the weighted total is computed in code, so the number is reproducible.
- **Pipeline:** a drag-and-drop board from Saved to Hired, with closed roles kept to the side.
- **Resume Studio:** a live preview of your resume in two templates (Clean, Modern), on A4 or US Letter, built with embedded Typst so no LaTeX install is needed.
  - **Tailor for this job:** the AI reorders and rewords your bullets for a specific posting. You accept or reject each change, and bullets it left out can be added back.
  - **Fact check:** companies, titles and dates are always copied from your profile. Any number or skill that isn't in your profile blocks export, and unfamiliar terms are flagged for a second look.
  - **ATS check:** the exported PDF is read back the way an applicant tracking system would, checking your name, section headings, role order, page count and keyword coverage for the posting. Every exported version is saved against its job.
- **Cover letters:** written for one job in three passes. A drafter writes, an independent reviewer acting as the hiring manager checks every claim against your profile and critiques it, then the draft is revised. You see the reviewer's notes, what changed, what was turned down and why, and how the letter handles each requirement in the posting (matched, honestly bridged, or a gap).
  - **Checks:** a number that isn't in your profile or the posting blocks export. A skill borrowed from the posting and claimed as yours is flagged, as are dashes, stock phrases and letters outside 250 to 350 words or over one page.
  - The letter uses the same header and template family as your resume, writes in the posting's language, and dates itself the local way. Every AI-written version is saved straight away with its notes.
- **Profile:** import a PDF, .txt or .md resume, or paste the text. The AI drafts your profile, and you review it before saving.
- **Settings:** bring your own key for Claude, OpenAI or OpenRouter, or run free and fully local with Ollama. Keys live in the OS keychain. A usage panel shows token spend across every AI call.
- **Everywhere:** ⌘K command palette, ⌘N to add any job by hand (for example from LinkedIn), ⌘1–6 to switch views, light and dark mode, and native window translucency on macOS and Windows.

## Layout

```
crates/core      domain model, SQLite storage + migrations, dedup keys, scoring
crates/sources   job sources: 10 ATS board types + 8 job searches; SSRF-guarded, throttled HTTP
crates/ai        Anthropic + OpenAI-compatible adapters, structured-output prompts
crates/docs      resume + cover letter models, Typst templates + renderer, fact gate, ATS read-back, tailoring
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

Render sample resumes and cover letters (PDF + SVG) without the UI:

```sh
cargo run -p waypoint-docs --example render -- /tmp
```

Check a live job source without the UI:

```sh
cargo run -p waypoint-sources --example fetch -- ashby supabase
```

Build installers for the current OS: `pnpm tauri build` produces a .dmg/.app on macOS, .msi/.exe on Windows, and .deb/.rpm/AppImage on Linux.

## Principles

- **Never invent facts.** Scores and generated text use only what's in your profile.
- **Never auto-submit.** Waypoint prepares and tracks; you apply.
- **Public data only.** It reads job boards companies publish for embedding and public job-search APIs, at most one request per second per site. It does not log in to or scrape LinkedIn or Indeed.
- **Local first.** The SQLite database lives in your app data folder and there's no Waypoint server.

See `NOTICE.md` for the open-source work this builds on.
