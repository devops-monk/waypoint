# Waypoint

An honest, private job search copilot for macOS, Windows and Linux. It finds real jobs on company job boards, scores how well you fit using only facts from your profile, and tracks every application. Your data stays on your computer, and AI calls use your own key.

## What works today

- **Companies:** follow a company by pasting its careers-page link. Waypoint reads Greenhouse, Lever, Ashby, Workday, SmartRecruiters, Workable, Recruitee, Personio and Teamtailor boards, and offers 129 suggestions to start from. Boards that fail or go empty are flagged. For Workday and SmartRecruiters, which list jobs without descriptions, the description is fetched the first time you open a job.
- **Job searches:** watch a keyword search on remote boards (Remotive, Himalayas, Remote OK), Hacker News' monthly "Who is hiring?" thread, national job boards (Arbeitsagentur in Germany, Platsbanken in Sweden, Jobnet in Denmark), or Adzuna for the US, UK, India and most of Europe (needs your own free Adzuna key, kept in the keychain).
- **Discover:** an inbox of new roles ranked by fit, with keyboard triage (J/K to move, S to save, X to skip), a remote filter, search, and a warning on old postings.
- **Fit scoring:** gates for eligibility, language and location, then four weighted dimensions (skills, experience, culture, career). Also covers strengths and gaps, ghost-job signals, and the keywords a posting screens for. The model scores each dimension and the weighted total is computed in code, so the number is reproducible.
- **Pipeline:** a drag-and-drop board from Saved to Hired, with closed roles kept to the side.
- **Insights:** in Pipeline, a funnel of how far applications get, applications per week, reply rate by source, the typical wait for a first answer, and whether your fit score actually predicts replies. Every chart can be shown as a table.
- **Follow-ups:** Waypoint tells you when to check in: a week after applying (twice, then the role is marked as gone quiet), within a day when a recruiter replies, and a thank-you the day after an interview. Draft the message with AI, copy it, send it yourself, and mark it as sent. Snooze a reminder or turn it off per job, change the timing in Settings, and get a system notification when one falls due while Waypoint is open.
- **Resume Studio:** a live preview of your resume in two templates (Clean, Modern), on A4 or US Letter, built with embedded Typst so no LaTeX install is needed.
  - **Tailor for this job:** the AI reorders and rewords your bullets for a specific posting. You accept or reject each change, and bullets it left out can be added back.
  - **Fact check:** companies, titles and dates are always copied from your profile. Any number or skill that isn't in your profile blocks export, and unfamiliar terms are flagged for a second look.
  - **ATS check:** the exported PDF is read back the way an applicant tracking system would, checking your name, section headings, role order, page count and keyword coverage for the posting. Every exported version is saved against its job. Export as PDF or as a Word document (.docx) for recruiters who ask for one.
- **Cover letters:** written for one job in three passes. A drafter writes, an independent reviewer acting as the hiring manager checks every claim against your profile and critiques it, then the draft is revised. You see the reviewer's notes, what changed, what was turned down and why, and how the letter handles each requirement in the posting (matched, honestly bridged, or a gap).
  - **Checks:** a number that isn't in your profile or the posting blocks export. A skill borrowed from the posting and claimed as yours is flagged, as are dashes, stock phrases and letters outside 250 to 350 words or over one page.
  - The letter uses the same header and template family as your resume, writes in the posting's language, and dates itself the local way. Every AI-written version is saved straight away with its notes.
- **Interviews:** a story bank of your experience in STAR form (the AI can draft stories from your profile, leaving [brackets] for details only you know), and a prep pack for each role: likely questions with why they're asked, the story to tell and talking points, honest answers for your gaps, questions to ask them and a checklist. Type a practice answer to any question for feedback against STAR and a stronger version built only from your facts. A debrief afterwards feeds the thank-you note.
- **Profile:** import a PDF, .txt or .md resume, or paste the text. The AI drafts your profile, and you review it before saving.
- **Settings:** bring your own key for Claude, OpenAI or OpenRouter, or run free and fully local with Ollama. Keys live in the OS keychain. A usage panel shows token spend across every AI call.
- **Everywhere:** ⌘K command palette, ⌘N to add any job by hand (for example from LinkedIn), ⌘1–7 to switch views, light and dark mode, and native window translucency on macOS and Windows.

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

## Releases

GitHub Actions builds every OS (`.github/workflows/release.yml`):

1. Bump `version` in `apps/desktop/src-tauri/tauri.conf.json` and the workspace `Cargo.toml`, and commit.
2. Push a matching tag: `git tag v0.1.0 && git push origin v0.1.0`.
3. The workflow builds macOS (Apple silicon and Intel), Windows and Linux, signs the update bundles, and attaches everything to a **draft** release. Check it on the Releases page.
4. Publish the release. `.github/workflows/publish.yml` then copies the installers to `waypoint.devops-monk.com/downloads/<version>/`, updates the website's download links, and writes the update feed at `/updates/latest.json`. Installed copies of Waypoint find the update on their next launch, verify its signature, and offer to install it (AppImage, macOS and Windows; `.deb` and `.rpm` users download the new package).

Update signing uses a key pair made with `tauri signer generate`: the public key is in `tauri.conf.json`, the private key is at `~/.tauri/waypoint-updater.key` (password in your macOS Keychain as `waypoint-updater-key-password`) and in the repo secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Keep a backup of it: without it, existing installs can't verify future updates. A local `pnpm tauri build` needs those two variables set too.

Every push and pull request also runs `.github/workflows/ci.yml`: Rust tests, clippy and the frontend type check.

Builds are unsigned for now, so macOS and Windows warn on first launch. To sign, add these repository secrets and pass them as `env` to the `tauri-action` step: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` and `APPLE_TEAM_ID` for a signed, notarized macOS build (needs an Apple Developer account), and a code-signing certificate for Windows.

## Website

`website/` is the static site at https://waypoint.devops-monk.com (one HTML file, screenshots in `website/assets`). Run `website/deploy.sh` to publish the page itself. Download links and the version shown come from `/downloads/manifest.json`, which the Publish workflow writes for each release. The server's nginx config lives at `/etc/nginx/sites-available/waypoint`, with HTTPS from certbot.

## Principles

- **Never invent facts.** Scores and generated text use only what's in your profile.
- **Never auto-submit.** Waypoint prepares and tracks; you apply.
- **Public data only.** It reads job boards companies publish for embedding and public job-search APIs, at most one request per second per site. It does not log in to or scrape LinkedIn or Indeed.
- **Local first.** The SQLite database lives in your app data folder and there's no Waypoint server.

See `NOTICE.md` for the open-source work this builds on.
