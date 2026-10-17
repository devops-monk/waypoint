# Notices

Waypoint ports logic and data from two MIT-licensed projects. The code was rewritten in Rust; credit and license terms follow.

## career-ops

Copyright (c) 2026 Santiago Fernández de Valderrama. MIT License. https://github.com/career-ops-hq/career-ops

Used as a reference for:
- the Greenhouse, Lever and Ashby providers (`crates/sources`)
- the application status model and aliases (`crates/core/src/status.rs`)
- the posting-legitimacy assessment in the evaluation prompt
- the starter company list (`crates/sources/src/seed_boards.json`, derived from `templates/portals.example.yml`)

"career-ops" is a trademark of its author. Waypoint is not affiliated with or endorsed by career-ops.

## ai-job-search

Copyright (c) 2026 Mads Lorentzen. MIT License. https://github.com/MadsLorentzen/ai-job-search

Used as a reference for:
- the dedup key algorithm (`crates/core/src/dedup.rs`, a port of `tools/job_key.py`)
- the fit-scoring weights, bands and gates (`crates/core/src/scoring.rs`, from `tools/rank_state.py` and `04-job-evaluation.md`)
- the tailoring rules and ATS read-back idea (`crates/docs`, from `05-cv-templates.md` and `tools/verify_pdf.py`)
- the cover letter draft, review and revise flow, structure and writing rules (`crates/ai/src/cover.rs`, `crates/docs/src/letter.rs`, from `.claude/commands/apply.md`, `03-writing-style.md` and `06-cover-letter-templates.md`)

## Inter typeface

Copyright (c) 2016 The Inter Project Authors. SIL Open Font License 1.1. https://github.com/rsms/inter. Bundled in `crates/docs/fonts/` with its license (`Inter-OFL.txt`) and embedded in exported resumes.

## Typst

The Typst compiler (Apache-2.0) is used as a library to lay out resumes. https://github.com/typst/typst

---

MIT License text (applies to career-ops and ai-job-search):

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
