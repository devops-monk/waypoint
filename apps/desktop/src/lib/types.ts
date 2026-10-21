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
  coverage: string;
  place: boolean;
  needs_key: string | null;
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

export interface LetterDoc {
  name: string;
  contact: string[];
  date: string;
  recipient: string[];
  subject: string;
  salutation: string;
  opening: string;
  highlights: string[];
  body: string[];
  closing: string;
  signature: string;
  paper: "a4" | "us-letter" | string;
  lang: string;
}

export interface LetterRequirement {
  requirement: string;
  status: "matched" | "bridged" | "gap" | string;
  evidence: string;
}

export interface LetterFinding {
  area: "grounding" | "requirements" | "company" | "tone" | "structure" | "length" | string;
  quote: string;
  problem: string;
  suggestion: string;
}

export interface LetterNotes {
  requirements: LetterRequirement[];
  review: { verdict: "ready" | "minor_fixes" | "needs_work" | string; overall: string; findings: LetterFinding[] };
  changes: string[];
  declined: string[];
  written_at: string;
  model: string;
}

export interface LetterState {
  doc: LetterDoc;
  template: Template;
  notes: LetterNotes | null;
  saved_at: string | null;
  exported_to: string | null;
  fit_warning: string | null;
}

export type FollowUpUrgency = "urgent" | "due" | "waiting" | "cold" | "off";
export type FollowUpAction = "follow_up" | "reply" | "thank_you";

export interface FollowUpItem {
  job_id: number;
  company: string;
  title: string;
  location: string;
  url: string;
  status: Status;
  status_at: string;
  days_in_status: number;
  snoozed: boolean;
  urgency: FollowUpUrgency;
  action: FollowUpAction;
  next_on: string | null;
  touches: number;
  last_touch: string | null;
}

export interface Cadence {
  applied_first: number;
  applied_subsequent: number;
  applied_max: number;
  responded_initial: number;
  responded_subsequent: number;
  interview_thankyou: number;
}

export interface FollowUpSettings {
  cadence: Cadence;
  notify: boolean;
}

export interface FollowUpBoard {
  items: FollowUpItem[];
  settings: FollowUpSettings;
  today: string;
}

export interface Touch {
  id: number;
  job_id: number;
  kind: FollowUpAction;
  channel: string;
  note: string;
  at: string;
}

export interface Story {
  id: number;
  title: string;
  source: string;
  situation: string;
  task: string;
  action: string;
  result: string;
  reflection: string;
  tags: string[];
  draft: boolean;
  updated_at: string | null;
}

export type QuestionCategory = "opener" | "motivation" | "behavioral" | "technical" | "curveball";

export interface PrepPack {
  role_focus: string[];
  company_brief: string;
  questions: { category: QuestionCategory; question: string; why: string; story_id: number; talking_points: string[] }[];
  gaps: { gap: string; answer: string }[];
  questions_to_ask: string[];
  checklist: string[];
}

export interface Feedback {
  score: number;
  summary: string;
  worked: string[];
  improve: string[];
  star: { situation: boolean; task: boolean; action: boolean; result: boolean };
  stronger_answer: string;
}

export interface Practice {
  id: number;
  job_id: number;
  question: string;
  answer: string;
  feedback: Feedback;
  created_at: string;
}

export interface PrepState {
  pack: PrepPack | null;
  done: string[];
  debrief: string;
  model: string;
  updated_at: string | null;
  practice: Practice[];
}

export interface Insights {
  funnel: { key: "saved" | "applied" | "heard_back" | "interview" | "offer" | "hired"; count: number }[];
  applied: number;
  heard_back: number;
  median_days_to_reply: number | null;
  active: number;
  by_source: { source: string; applied: number; heard_back: number }[];
  weeks: { start: string; applied: number }[];
  fit: { heard_back_avg: number; heard_back_n: number; silent_avg: number; silent_n: number } | null;
}
