// Mirrors of the Rust types serialized over Tauri IPC.

export type Status =
  | "new"
  | "saved"
  | "evaluated"
  | "applied"
  | "responded"
  | "assessment"
  | "interview"
  | "offer"
  | "hired"
  | "rejected"
  | "discarded"
  | "skip";

export interface Salary {
  min: number | null;
  max: number | null;
  currency: string | null;
  period: string | null;
}

export interface JobSummary {
  id: number;
  company: string;
  title: string;
  location: string;
  remote: boolean;
  source: string;
  url: string;
  salary: Salary | null;
  posted_at: string | null;
  first_seen_at: string;
  status: Status;
  starred: boolean;
  score: number | null;
  band: string | null;
}

export interface Job extends JobSummary {
  dedup_key: string;
  board: string | null;
  employment_type: string | null;
  department: string | null;
  description: string;
  last_seen_at: string;
  liveness: string;
  status_at: string;
  notes: string;
}

export interface JobFilter {
  statuses?: Status[] | null;
  query?: string | null;
  remote_only?: boolean;
  starred_only?: boolean;
  min_score?: number | null;
  limit?: number | null;
}

export interface Board {
  id: number;
  provider: string;
  slug: string;
  company: string;
  enabled: boolean;
  last_run_at: string | null;
  last_error: string | null;
  last_count: number | null;
}

export interface SeedBoard {
  provider: string;
  slug: string;
  company: string;
}

export interface ProviderInfo {
  id: string;
  name: string;
  kind: "board" | "search";
  example: string;
}

export interface ScanProgress {
  done: number;
  total: number;
  company: string;
  found: number;
  error: string | null;
}

export interface ScanSummary {
  boards: number;
  fetched: number;
  new_jobs: number;
  errors: number;
}

export interface Experience {
  company: string;
  title: string;
  location: string;
  start: string;
  end: string;
  highlights: string[];
}

export interface Education {
  institution: string;
  degree: string;
  start: string;
  end: string;
}

export interface Preferences {
  target_roles: string[];
  locations: string[];
  remote_ok: boolean;
  relocation_ok: boolean;
  salary_floor: number | null;
  salary_currency: string;
  work_authorization: string;
  needs_sponsorship: boolean;
  career_goals: string[];
  energizing: string[];
  draining: string[];
  deal_breakers: string[];
}

export interface Profile {
  name: string;
  email: string;
  headline: string;
  location: string;
  summary: string;
  links: string[];
  experience: Experience[];
  education: Education[];
  skills: string[];
  languages: { name: string; level: string }[];
  preferences: Preferences;
}

export type AiProvider = "anthropic" | "openai" | "openrouter" | "ollama";

export interface LlmConfig {
  provider: AiProvider;
  model: string;
  base_url?: string | null;
  effort?: string | null;
}

export interface AiSettings {
  config: LlmConfig | null;
  keys: [AiProvider, boolean][];
}

export type GateVerdict = "PASS" | "FLAG" | "FAIL";

export interface Evaluation {
  summary: string;
  gates: Record<"eligibility" | "language" | "location", { verdict: GateVerdict; note: string }>;
  dimensions: Record<"technical" | "experience" | "behavioral" | "career", { score: number; rationale: string }>;
  strengths: string[];
  gaps: string[];
  red_flags: string[];
  legitimacy: { risk: "low" | "medium" | "high"; signals: string[] };
  keywords: string[];
}

export interface EvaluationView {
  stored: {
    id: number;
    job_id: number;
    score: number | null;
    band: string;
    model: string;
    input_tokens: number;
    output_tokens: number;
    created_at: string;
  };
  evaluation: Evaluation | null;
}

export interface UsageRow {
  model: string;
  calls: number;
  input_tokens: number;
  output_tokens: number;
}

export interface PipelineCount {
  status: Status;
  count: number;
}

// ── Resume Studio ───────────────────────────────────────────────────────

export type Template = "clean" | "modern";

export interface ResumeRole {
  company: string;
  title: string;
  location: string;
  start: string;
  end: string;
  bullets: string[];
}

export interface ResumeDoc {
  name: string;
  headline: string;
  contact: string[];
  summary: string;
  experience: ResumeRole[];
  education: { institution: string; degree: string; start: string; end: string }[];
  skills: string[];
  languages: string[];
  paper: "a4" | "us-letter" | string;
}

export interface FactIssue {
  severity: "block" | "warn";
  path: string;
  message: string;
}

export interface ResumeState {
  doc: ResumeDoc;
  template: Template;
  saved_at: string | null;
  exported_to: string | null;
}

export interface BulletChange {
  source_index: number;
  original: string;
  proposed: string;
  issues: FactIssue[];
}

export interface Proposal {
  summary_original: string;
  summary_proposed: string;
  summary_issues: FactIssue[];
  roles: { role_index: number; company: string; title: string; bullets: BulletChange[]; omitted: [number, string][] }[];
  skills_proposed: string[];
  skill_issues: FactIssue[];
  rationale: string;
  doc: ResumeDoc;
}

export interface AtsReport {
  pages: number;
  extracted_chars: number;
  name_found: boolean;
  missing_sections: string[];
  companies_in_order: boolean;
  keywords: { keyword: string; found: boolean }[];
  coverage: number;
  problems: string[];
}
