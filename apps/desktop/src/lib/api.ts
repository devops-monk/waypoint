import { invoke } from "@tauri-apps/api/core";
import type {
  AiProvider,
  AiSettings,
  Board,
  EvaluationView,
  Job,
  JobFilter,
  JobSummary,
  LlmConfig,
  PipelineCount,
  Profile,
  ProviderInfo,
  ScanSummary,
  SeedBoard,
  Status,
  UsageRow,
  AtsReport,
  Proposal,
  ResumeDoc,
  ResumeState,
  FactIssue,
  Template,
  LetterDoc,
  LetterState,
  FollowUpAction,
  FollowUpBoard,
  FollowUpSettings,
  Touch,
  Story,
  PrepState,
  Practice,
  Insights,
} from "./types";

// One typed wrapper per Rust command. Argument names follow Tauri's camelCase convention.
export const api = {
  listJobs: (filter: JobFilter) => invoke<JobSummary[]>("list_jobs", { filter }),
  getJob: (id: number) => invoke<Job>("get_job", { id }),
  setStatus: (id: number, status: Status, note?: string) => invoke<void>("set_status", { id, status, note }),
  setStarred: (id: number, starred: boolean) => invoke<void>("set_starred", { id, starred }),
  setNotes: (id: number, notes: string) => invoke<void>("set_notes", { id, notes }),
  pipelineCounts: () => invoke<PipelineCount[]>("pipeline_counts"),
  addJobManual: (job: { url: string; company: string; title: string; location: string; description: string }) =>
    invoke<number>("add_job_manual", { job }),

  providers: () => invoke<ProviderInfo[]>("providers"),
  seedBoards: () => invoke<SeedBoard[]>("seed_boards"),
  listBoards: () => invoke<Board[]>("list_boards"),
  detectBoard: (url: string) => invoke<{ provider: string; slug: string } | null>("detect_board", { url }),
  addBoard: (provider: string, slug: string, company: string) => invoke<number>("add_board", { provider, slug, company }),
  removeBoard: (id: number) => invoke<void>("remove_board", { id }),
  setBoardEnabled: (id: number, enabled: boolean) => invoke<void>("set_board_enabled", { id, enabled }),
  scanAll: () => invoke<ScanSummary>("scan_all"),
  sourceKeyStatus: () => invoke<{ adzuna: boolean }>("source_key_status"),
  saveAdzunaKey: (appId: string, appKey: string) => invoke<void>("save_adzuna_key", { appId, appKey }),
  deleteAdzunaKey: () => invoke<void>("delete_adzuna_key"),

  getProfile: () => invoke<Profile>("get_profile"),
  saveProfile: (profile: Profile) => invoke<number>("save_profile", { profile }),
  importResume: (args: { path?: string; text?: string }) => invoke<Profile>("import_resume", args),

  getAiSettings: () => invoke<AiSettings>("get_ai_settings"),
  saveAiSettings: (config: LlmConfig, apiKey?: string) => invoke<void>("save_ai_settings", { config, apiKey }),
  deleteApiKey: (provider: AiProvider) => invoke<void>("delete_api_key", { provider }),

  latestEvaluation: (jobId: number) => invoke<EvaluationView | null>("latest_evaluation", { jobId }),
  evaluateJob: (jobId: number) => invoke<EvaluationView>("evaluate_job", { jobId }),
  tokenUsage: () => invoke<UsageRow[]>("token_usage"),
  insights: () => invoke<Insights>("insights"),

  resumeLoad: (jobId: number | null) => invoke<ResumeState>("resume_load", { jobId }),
  resumeRender: (doc: ResumeDoc, template: Template) => invoke<{ pages: string[]; issues: FactIssue[] }>("resume_render", { doc, template }),
  resumeTailor: (jobId: number) => invoke<Proposal>("resume_tailor", { jobId }),
  resumeSave: (jobId: number | null, doc: ResumeDoc, template: Template) => invoke<number>("resume_save", { jobId, doc, template }),
  resumeExport: (jobId: number | null, doc: ResumeDoc, template: Template, path: string, format: "pdf" | "docx" = "pdf") =>
    invoke<{ path: string; ats: AtsReport }>("resume_export", { req: { job_id: jobId, doc, template, path, format } }),
  resumeAts: (jobId: number | null, doc: ResumeDoc, template: Template) => invoke<AtsReport>("resume_ats", { jobId, doc, template }),

  followups: () => invoke<FollowUpBoard>("followups"),
  followupLog: (jobId: number, kind: FollowUpAction, channel: string, note: string) => invoke<number>("followup_log", { jobId, kind, channel, note }),
  followupHistory: (jobId: number) => invoke<Touch[]>("followup_history", { jobId }),
  followupDelete: (id: number) => invoke<void>("followup_delete", { id }),
  followupSnooze: (jobId: number, days: number | null) => invoke<void>("followup_snooze", { jobId, days }),
  followupSetOff: (jobId: number, off: boolean) => invoke<void>("followup_set_off", { jobId, off }),
  saveFollowupSettings: (settings: FollowUpSettings) => invoke<void>("save_followup_settings", { settings }),
  followupDraft: (jobId: number, action: FollowUpAction, notes: string) => invoke<{ subject: string; body: string }>("followup_draft", { jobId, action, notes }),

  storiesList: () => invoke<Story[]>("stories_list"),
  storySave: (story: Story) => invoke<number>("story_save", { story }),
  storyDelete: (id: number) => invoke<void>("story_delete", { id }),
  storiesSuggest: () => invoke<{ added: number; flagged: string[] }>("stories_suggest"),
  prepGet: (jobId: number) => invoke<PrepState>("prep_get", { jobId }),
  prepBuild: (jobId: number) => invoke<PrepState>("prep_build", { jobId }),
  prepNotes: (jobId: number, done: string[], debrief: string) => invoke<void>("prep_notes", { jobId, done, debrief }),
  practiceAnswer: (jobId: number, question: string, answer: string) => invoke<Practice>("practice_answer", { jobId, question, answer }),

  letterLoad: (jobId: number) => invoke<LetterState>("letter_load", { jobId }),
  letterWrite: (jobId: number) => invoke<LetterState>("letter_write", { jobId }),
  letterRender: (jobId: number, doc: LetterDoc, template: Template) =>
    invoke<{ pages: string[]; issues: FactIssue[]; words: number }>("letter_render", { jobId, doc, template }),
  letterSave: (jobId: number, doc: LetterDoc, template: Template) => invoke<number>("letter_save", { jobId, doc, template }),
  letterExport: (jobId: number, doc: LetterDoc, template: Template, path: string, format: "pdf" | "docx" = "pdf") =>
    invoke<string>("letter_export", { req: { job_id: jobId, doc, template, path, format } }),
};

export function errorMessage(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return "Something went wrong.";
}
