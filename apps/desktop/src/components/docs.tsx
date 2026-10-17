import clsx from "clsx";
import { AlertTriangle, ShieldCheck } from "lucide-react";
import type { FactIssue, JobSummary } from "../lib/types";
import { EXITS } from "../lib/format";

/** Shared pieces of the Documents view: resume and cover letter. */

export const blocked = (issues: FactIssue[]) => issues.some((i) => i.severity === "block");

export function IssueList({ issues }: { issues: FactIssue[] }) {
  if (!issues.length) return null;
  return (
    <ul className="mt-1.5 flex flex-col gap-1">
      {issues.map((i, k) => (
        <li key={k} className={clsx("flex items-start gap-1.5 text-caption", i.severity === "block" ? "text-coral" : "text-amber")}>
          <AlertTriangle className="mt-px size-3 shrink-0" />
          {i.message}
        </li>
      ))}
    </ul>
  );
}

/** The fact-check summary above the editor. */
export function CheckBanner({ issues, clean, extra }: { issues: FactIssue[]; clean: string; extra?: string }) {
  const blocks = issues.filter((i) => i.severity === "block").length;
  const warns = issues.length - blocks;
  return (
    <div className={clsx("mb-5 flex items-center gap-2 rounded-[9px] px-3 py-2 text-callout", blocks ? "bg-coral/10 text-coral" : warns ? "bg-amber/10 text-amber" : "bg-teal/10 text-teal")}>
      {blocks ? <AlertTriangle className="size-3.5 shrink-0" /> : <ShieldCheck className="size-3.5 shrink-0" />}
      <span className="flex-1">
        {blocks
          ? `${blocks} ${blocks === 1 ? "claim isn't" : "claims aren't"} backed by your profile. Fix ${blocks === 1 ? "it" : "them"} to export.`
          : warns
            ? `Every fact checks out. ${warns} ${warns === 1 ? "thing is" : "things are"} worth a second look.`
            : clean}
      </span>
      {extra && <span className="tabular text-caption opacity-80">{extra}</span>}
    </div>
  );
}

export function PagePreview({ pages, error }: { pages: string[]; error: string | null }) {
  return (
    <div className="scroll min-w-0 flex-1 bg-fill">
      {error ? (
        <p className="m-8 rounded-[9px] bg-coral/10 p-3 text-callout text-coral">{error}</p>
      ) : (
        <div className="mx-auto flex max-w-[720px] flex-col gap-6 px-8 py-8">
          {pages.map((svg, i) => (
            <div
              key={i}
              className="overflow-hidden rounded-[3px] bg-white shadow-[0_0_0_0.5px_rgb(0_0_0/0.08),0_8px_24px_rgb(0_0_0/0.12)] [&>svg]:block [&>svg]:h-auto [&>svg]:w-full"
              // Typst output: vector pages, text drawn as glyph paths.
              dangerouslySetInnerHTML={{ __html: svg }}
            />
          ))}
          {pages.length > 1 && <p className="text-center text-caption text-ink-3">{pages.length} pages</p>}
        </div>
      )}
    </div>
  );
}

export function JobPicker({ jobs, jobId, onChange, baseLabel }: { jobs: JobSummary[]; jobId: number | null; onChange: (id: number | null) => void; baseLabel?: string }) {
  return (
    <select
      value={jobId ?? ""}
      onChange={(e) => onChange(e.target.value ? Number(e.target.value) : null)}
      className="h-7 max-w-[280px] min-w-0 rounded-control bg-surface px-2 text-callout text-ink shadow-[0_0_0_0.5px_var(--color-hairline),0_1px_1.5px_rgb(0_0_0/0.08)] outline-none"
      aria-label="Document for"
    >
      {baseLabel ? <option value="">{baseLabel}</option> : jobId == null && <option value="">Choose a job…</option>}
      {jobs.filter((j) => !EXITS.includes(j.status)).map((j) => (
        <option key={j.id} value={j.id}>{j.title} at {j.company}</option>
      ))}
    </select>
  );
}
