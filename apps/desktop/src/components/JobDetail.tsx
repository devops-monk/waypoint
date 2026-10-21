import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import clsx from "clsx";
import { AlertTriangle, Bookmark, Check, ExternalLink, FileText, Mail, MessagesSquare, Sparkles, Star, X } from "lucide-react";
import type { DocKind } from "../views/Documents";
import { motion } from "motion/react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { api, errorMessage } from "../lib/api";
import { SOURCE_LABEL, ageInDays, formatSalary, humanize, relativeTime } from "../lib/format";
import type { Evaluation, GateVerdict, Status } from "../lib/types";
import { FitDial } from "./FitDial";
import { PeopleForJob } from "./People";
import { RouteTrail } from "./RouteTrail";
import { Button, IconButton, Panel, SectionTitle, Tag, Textarea } from "./ui";

const GATE_LABEL = { eligibility: "Work eligibility", language: "Language", location: "Location" } as const;
const DIM_LABEL = { technical: "Skills", experience: "Experience", behavioral: "Culture", career: "Career goals" } as const;

function GateChip({ name, verdict, note }: { name: keyof typeof GATE_LABEL; verdict: GateVerdict; note: string }) {
  const tone = verdict === "PASS" ? "teal" : verdict === "FLAG" ? "amber" : "coral";
  return (
    <div className="flex min-w-0 flex-1 flex-col gap-1 rounded-[9px] bg-fill px-3 py-2" title={note}>
      <div className="flex items-center justify-between gap-2">
        <span className="text-callout font-medium">{GATE_LABEL[name]}</span>
        <Tag tone={tone}>{verdict === "PASS" ? "Clear" : verdict === "FLAG" ? "Check" : "Blocker"}</Tag>
      </div>
      {note && <span className="selectable line-clamp-2 text-caption text-ink-2">{note}</span>}
    </div>
  );
}

function DimensionBar({ label, score, rationale }: { label: string; score: number; rationale: string }) {
  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-baseline justify-between">
        <span className="text-callout font-medium">{label}</span>
        <span className="tabular text-callout text-ink-2">{Math.round(score)}</span>
      </div>
      <div className="h-1.5 overflow-hidden rounded-full bg-fill-strong">
        <motion.div className="h-full rounded-full bg-gradient-to-r from-indigo to-teal" initial={{ width: 0 }} animate={{ width: `${Math.max(0, Math.min(100, score))}%` }} transition={{ type: "spring", stiffness: 140, damping: 24 }} />
      </div>
      <p className="selectable text-caption leading-relaxed text-ink-2">{rationale}</p>
    </div>
  );
}

function BulletList({ items, tone }: { items: string[]; tone: "teal" | "amber" | "coral" }) {
  if (!items.length) return <p className="text-callout text-ink-3">None noted.</p>;
  return (
    <ul className="selectable flex flex-col gap-1.5">
      {items.map((s, i) => (
        <li key={i} className="flex gap-2 text-callout leading-relaxed text-ink">
          <span className={clsx("mt-[7px] size-1.5 shrink-0 rounded-full", tone === "teal" ? "bg-teal" : tone === "amber" ? "bg-amber" : "bg-coral")} />
          {s}
        </li>
      ))}
    </ul>
  );
}

function EvaluationPanel({ evaluation, score, band, model }: { evaluation: Evaluation; score: number | null; band: string; model: string }) {
  const risk = evaluation.legitimacy.risk;
  return (
    <div className="flex flex-col gap-5">
      <div className="flex items-center gap-6">
        <FitDial score={score} band={band} />
        <div className="min-w-0 flex-1">
          <p className="selectable text-body leading-relaxed text-ink">{evaluation.summary}</p>
          <p className="mt-2 text-caption text-ink-3">Scored by {model} against your profile</p>
        </div>
      </div>
      <div className="flex gap-2">
        {(Object.keys(GATE_LABEL) as (keyof typeof GATE_LABEL)[]).map((g) => (
          <GateChip key={g} name={g} verdict={evaluation.gates[g].verdict} note={evaluation.gates[g].note} />
        ))}
      </div>
      <div className="grid grid-cols-2 gap-x-6 gap-y-4">
        {(Object.keys(DIM_LABEL) as (keyof typeof DIM_LABEL)[]).map((d) => (
          <DimensionBar key={d} label={DIM_LABEL[d]} score={evaluation.dimensions[d].score} rationale={evaluation.dimensions[d].rationale} />
        ))}
      </div>
      <div className="grid grid-cols-2 gap-6">
        <div>
          <SectionTitle>Why you fit</SectionTitle>
          <BulletList items={evaluation.strengths} tone="teal" />
        </div>
        <div>
          <SectionTitle>Gaps to address</SectionTitle>
          <BulletList items={evaluation.gaps} tone="amber" />
        </div>
      </div>
      {(risk !== "low" || evaluation.red_flags.length > 0) && (
        <div className="rounded-[10px] bg-amber/10 p-3.5">
          <p className="flex items-center gap-2 text-callout font-semibold text-amber">
            <AlertTriangle className="size-4" />
            {risk === "high" ? "This may be a ghost job" : risk === "medium" ? "Some signs this posting may not be active" : "Worth checking before you apply"}
          </p>
          <div className="mt-2">
            <BulletList items={[...evaluation.legitimacy.signals, ...evaluation.red_flags]} tone="amber" />
          </div>
        </div>
      )}
      {evaluation.keywords.length > 0 && (
        <div>
          <SectionTitle>Keywords this posting screens for</SectionTitle>
          <div className="flex flex-wrap gap-1.5">
            {evaluation.keywords.map((k) => (
              <Tag key={k}>{k}</Tag>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

export function JobDetail({ jobId, onTriage, onOpenDocs, onPrepare }: { jobId: number; onTriage?: (s: Status) => void; onOpenDocs?: (kind: DocKind) => void; onPrepare?: () => void }) {
  const qc = useQueryClient();
  const { data: job } = useQuery({ queryKey: ["job", jobId], queryFn: () => api.getJob(jobId) });
  const { data: ev } = useQuery({ queryKey: ["evaluation", jobId], queryFn: () => api.latestEvaluation(jobId) });
  const [notes, setNotes] = useState("");
  useEffect(() => setNotes(job?.notes ?? ""), [job?.notes]);

  const evaluate = useMutation({
    mutationFn: () => api.evaluateJob(jobId),
    onSuccess: () => qc.invalidateQueries(),
    onError: (e) => toast.error(errorMessage(e)),
  });

  const move = async (s: Status) => {
    await api.setStatus(jobId, s);
    await qc.invalidateQueries();
  };

  if (!job) return <div className="h-full" />;
  const salary = formatSalary(job.salary);
  const age = ageInDays(job.posted_at);

  return (
    <div className="scroll h-full">
      <article className="mx-auto max-w-[760px] px-8 pt-7 pb-16">
        <div className="flex items-start gap-4">
          <div className="min-w-0 flex-1">
            <p className="text-body font-medium text-ink-2">{job.company}</p>
            <h2 className="selectable mt-0.5 text-title font-semibold text-balance">{job.title}</h2>
            <div className="mt-2.5 flex flex-wrap items-center gap-1.5">
              {job.location && <Tag>{job.location}</Tag>}
              {job.remote && <Tag tone="teal">Remote</Tag>}
              {job.employment_type && <Tag>{humanize(job.employment_type)}</Tag>}
              {salary && <Tag tone="indigo">{salary}</Tag>}
              <span className="ml-1 text-caption text-ink-3">
                {job.posted_at ? `Posted ${relativeTime(job.posted_at).toLowerCase()}` : `Found ${relativeTime(job.first_seen_at).toLowerCase()}`} on {SOURCE_LABEL[job.source] ?? job.source}
              </span>
            </div>
          </div>
          <div className="flex shrink-0 items-center gap-1">
            <IconButton
              label={job.starred ? "Remove star" : "Star"}
              onClick={async () => {
                await api.setStarred(jobId, !job.starred);
                qc.invalidateQueries();
              }}
            >
              <Star className={clsx("size-4", job.starred && "fill-amber text-amber")} />
            </IconButton>
            {!job.url.startsWith("manual://") && (
              <IconButton label="Open posting" onClick={() => openUrl(job.url)}>
                <ExternalLink className="size-4" />
              </IconButton>
            )}
          </div>
        </div>

        <div className="mt-5 flex items-center justify-between gap-4">
          <RouteTrail status={job.status} onMove={move} />
          {onTriage && job.status === "new" && (
            <div className="flex gap-1.5">
              <Button size="sm" variant="plain" icon={<X className="size-3.5" />} onClick={() => onTriage("skip")}>Skip</Button>
              <Button size="sm" icon={<Bookmark className="size-3.5" />} onClick={() => onTriage("saved")}>Save</Button>
            </div>
          )}
        </div>

        {age != null && age > 45 && (
          <p className="mt-4 flex items-center gap-2 rounded-[9px] bg-amber/10 px-3 py-2 text-callout text-amber">
            <AlertTriangle className="size-4 shrink-0" />
            Posted {age} days ago. Long-open roles are often filled or paused, so confirm it's active before tailoring.
          </p>
        )}

        <Panel className="mt-6 p-5">
          {ev?.evaluation ? (
            <>
              <EvaluationPanel evaluation={ev.evaluation} score={ev.stored.score} band={ev.stored.band} model={ev.stored.model} />
              <div className="mt-5 flex justify-end gap-2">
                <Button size="sm" variant="plain" busy={evaluate.isPending} icon={<Sparkles className="size-3.5" />} onClick={() => evaluate.mutate()}>
                  Score again
                </Button>
                {onPrepare && ["responded", "assessment", "interview"].includes(job.status) && (
                  <Button size="sm" icon={<MessagesSquare className="size-3.5" />} onClick={onPrepare}>Prepare for interview</Button>
                )}
                {onOpenDocs && (
                  <>
                    <Button size="sm" icon={<Mail className="size-3.5" />} onClick={async () => {
                      if (job.status === "new") await move("saved");
                      onOpenDocs("letter");
                    }}>
                      Cover letter
                    </Button>
                    <Button size="sm" variant="primary" icon={<FileText className="size-3.5" />} onClick={async () => {
                      if (job.status === "new") await move("saved");
                      onOpenDocs("resume");
                    }}>
                      Tailor resume
                    </Button>
                  </>
                )}
              </div>
            </>
          ) : (
            <div className="flex items-center gap-5">
              <FitDial score={null} band={null} size={88} />
              <div className="flex-1">
                <p className="text-body font-semibold">How well do you fit?</p>
                <p className="mt-1 text-callout text-ink-2">Waypoint checks eligibility, language and location first, then scores skills, experience, culture and career fit using only facts from your profile.</p>
              </div>
              <Button variant="primary" busy={evaluate.isPending} icon={<Sparkles className="size-3.5" />} onClick={() => evaluate.mutate()}>
                {evaluate.isPending ? "Scoring…" : "Score this job"}
              </Button>
            </div>
          )}
        </Panel>

        <PeopleForJob jobId={jobId} company={job.company} />

        <section className="mt-8">
          <SectionTitle action={notes !== (job.notes ?? "") ? <span className="flex items-center gap-1 text-caption text-ink-3">Saves when you click away</span> : job.notes ? <Check className="size-3.5 text-teal" /> : null}>
            Notes
          </SectionTitle>
          <Textarea
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            onBlur={async () => {
              if (notes !== job.notes) {
                await api.setNotes(jobId, notes);
                qc.invalidateQueries({ queryKey: ["job", jobId] });
              }
            }}
            placeholder="Contacts, referral, what you liked, questions to ask"
            className="min-h-20 text-callout"
          />
        </section>

        <section className="mt-8">
          <SectionTitle>Job description</SectionTitle>
          {job.description ? (
            <div className="selectable max-w-[68ch] text-body leading-[1.65] whitespace-pre-wrap text-ink">{job.description}</div>
          ) : (
            <p className="text-callout text-ink-3">This source didn't include a description. Open the posting to read it.</p>
          )}
        </section>
      </article>
    </div>
  );
}
