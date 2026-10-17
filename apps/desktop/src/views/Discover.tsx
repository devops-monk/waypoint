import { useQuery, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import clsx from "clsx";
import { Building2, KeyRound, RefreshCw, Search, Star, UserRound } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { FitBadge } from "../components/FitDial";
import { JobDetail } from "../components/JobDetail";
import type { View } from "../components/Sidebar";
import { Button, Input, Segmented, Toggle } from "../components/ui";
import { api, errorMessage } from "../lib/api";
import { ageInDays, formatSalary, relativeTime } from "../lib/format";
import type { JobFilter, JobSummary, ScanProgress, Status } from "../lib/types";

type Scope = "inbox" | "shortlist" | "all";

const SCOPE_STATUSES: Record<Scope, Status[] | null> = {
  inbox: ["new"],
  shortlist: ["saved", "evaluated"],
  all: null,
};

function useScan() {
  const qc = useQueryClient();
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [running, setRunning] = useState(false);

  useEffect(() => {
    const un = listen<ScanProgress>("scan://progress", (e) => setProgress(e.payload));
    return () => {
      un.then((f) => f());
    };
  }, []);

  const run = useCallback(async () => {
    setRunning(true);
    setProgress(null);
    try {
      const s = await api.scanAll();
      if (s.boards === 0) toast("Pick some companies to follow first", { description: "Open Companies to choose where Waypoint looks." });
      else toast.success(s.new_jobs ? `Found ${s.new_jobs} new ${s.new_jobs === 1 ? "job" : "jobs"}` : "No new jobs since the last check", { description: s.errors ? `${s.errors} ${s.errors === 1 ? "company" : "companies"} couldn't be reached. See Companies for details.` : undefined });
      await qc.invalidateQueries();
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setRunning(false);
      setProgress(null);
    }
  }, [qc]);

  return { run, running, progress };
}

function JobRow({ job, selected, onSelect }: { job: JobSummary; selected: boolean; onSelect: () => void }) {
  const salary = formatSalary(job.salary);
  const age = ageInDays(job.posted_at);
  return (
    <button
      onClick={onSelect}
      data-selected={selected}
      className={clsx(
        "group relative flex w-full flex-col gap-0.5 rounded-[9px] px-3 py-2.5 text-left transition-colors",
        selected ? "bg-selection text-white" : "hover:bg-fill",
      )}
    >
      <div className="flex items-center gap-2">
        <span className={clsx("min-w-0 flex-1 truncate text-callout font-medium", selected ? "text-white/85" : "text-ink-2")}>{job.company}</span>
        {job.starred && <Star className={clsx("size-3 fill-current", selected ? "text-white" : "text-amber")} />}
        <span className={clsx("tabular text-caption", selected ? "text-white/75" : age != null && age > 45 ? "text-amber" : "text-ink-3")}>{relativeTime(job.posted_at ?? job.first_seen_at)}</span>
      </div>
      <div className="flex items-start gap-2">
        <span className={clsx("min-w-0 flex-1 text-body leading-snug font-semibold", selected ? "text-white" : "text-ink")}>{job.title}</span>
        <FitBadge score={job.score} band={job.band} />
      </div>
      <div className={clsx("flex min-w-0 items-center gap-2 text-callout", selected ? "text-white/80" : "text-ink-3")}>
        <span className="truncate">{job.location || "Location not listed"}</span>
        {salary && <span className="shrink-0">{salary}</span>}
      </div>
    </button>
  );
}

function GettingStarted({ onGoTo, onScan, scanning }: { onGoTo: (v: View) => void; onScan: () => void; scanning: boolean }) {
  const { data: boards } = useQuery({ queryKey: ["boards"], queryFn: api.listBoards });
  const { data: profile } = useQuery({ queryKey: ["profile"], queryFn: api.getProfile });
  const { data: ai } = useQuery({ queryKey: ["ai-settings"], queryFn: api.getAiSettings });
  const steps = [
    { done: (boards?.length ?? 0) > 0, icon: <Building2 className="size-4" />, title: "Choose companies to follow", body: "Waypoint checks their public job boards directly, so you see roles the day they're posted.", action: () => onGoTo("sources"), cta: "Choose companies" },
    { done: !!profile && (profile.experience.length > 0 || profile.skills.length > 0), icon: <UserRound className="size-4" />, title: "Add your profile", body: "Import your resume. Every score and tailored resume is built only from what's here.", action: () => onGoTo("profile"), cta: "Add profile" },
    { done: !!ai?.config, icon: <KeyRound className="size-4" />, title: "Connect an AI model", body: "Use your own Claude, OpenAI or OpenRouter key, or a free local model with Ollama.", action: () => onGoTo("settings"), cta: "Connect" },
  ];
  return (
    <div className="mx-auto flex max-w-[460px] flex-col justify-center px-6 py-16">
      <h2 className="text-title font-semibold">Set up your search</h2>
      <p className="mt-2 mb-7 text-body text-ink-2">Three steps, about three minutes. Everything stays on this computer.</p>
      <ol className="flex flex-col">
        {steps.map((s, i) => (
          <li key={s.title} className="relative flex gap-4 pb-6 last:pb-0">
            {i < steps.length - 1 && <span aria-hidden className="absolute top-8 bottom-1 left-[15px] border-l-2 border-dotted border-fill-strong" />}
            <span className={clsx("z-10 flex size-8 shrink-0 items-center justify-center rounded-full", s.done ? "bg-teal text-white" : "bg-fill text-ink-2")}>{s.icon}</span>
            <div className="flex-1 pt-1">
              <p className={clsx("text-body font-semibold", s.done && "text-ink-3 line-through decoration-ink-3/50")}>{s.title}</p>
              <p className="mt-0.5 text-callout text-ink-2">{s.body}</p>
              {!s.done && <Button size="sm" className="mt-2.5" onClick={s.action}>{s.cta}</Button>}
            </div>
          </li>
        ))}
      </ol>
      {steps[0].done && (
        <Button variant="primary" className="mt-8 self-start" busy={scanning} icon={<RefreshCw className="size-3.5" />} onClick={onScan}>
          Check for jobs now
        </Button>
      )}
    </div>
  );
}

export function Discover({ focusJob, onFocusHandled, onGoTo, onTailor }: { focusJob: number | null; onFocusHandled: () => void; onGoTo: (v: View) => void; onTailor: (id: number) => void }) {
  const qc = useQueryClient();
  const [scope, setScope] = useState<Scope>("inbox");
  const [query, setQuery] = useState("");
  const [remoteOnly, setRemoteOnly] = useState(false);
  const [selected, setSelected] = useState<number | null>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const scan = useScan();

  const filter: JobFilter = useMemo(() => ({ statuses: SCOPE_STATUSES[scope], query: query || null, remote_only: remoteOnly, limit: 400 }), [scope, query, remoteOnly]);
  const { data: jobs = [], isLoading } = useQuery({ queryKey: ["jobs", filter], queryFn: () => api.listJobs(filter) });
  const { data: anyJobs } = useQuery({ queryKey: ["jobs", "any"], queryFn: () => api.listJobs({ limit: 1 }) });

  useEffect(() => {
    if (focusJob != null) {
      setScope("all");
      setSelected(focusJob);
      onFocusHandled();
    }
  }, [focusJob, onFocusHandled]);

  useEffect(() => {
    if (selected == null && jobs.length) setSelected(jobs[0].id);
  }, [jobs, selected]);

  const move = useCallback(
    (delta: number) => {
      if (!jobs.length) return;
      const i = Math.max(0, jobs.findIndex((j) => j.id === selected));
      const next = jobs[Math.min(jobs.length - 1, Math.max(0, i + delta))];
      setSelected(next.id);
      listRef.current?.querySelector(`[data-id="${next.id}"]`)?.scrollIntoView({ block: "nearest" });
    },
    [jobs, selected],
  );

  const triage = useCallback(
    async (status: Status) => {
      if (selected == null) return;
      const i = jobs.findIndex((j) => j.id === selected);
      await api.setStatus(selected, status);
      // Keep the cursor in place, so triage flows down the list.
      const next = jobs[i + 1] ?? jobs[i - 1];
      setSelected(next?.id ?? null);
      await qc.invalidateQueries();
    },
    [jobs, selected, qc],
  );

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement;
      if (t.closest("input, textarea, [contenteditable]") || e.metaKey || e.ctrlKey || e.altKey) return;
      if (e.key === "j" || e.key === "ArrowDown") (e.preventDefault(), move(1));
      else if (e.key === "k" || e.key === "ArrowUp") (e.preventDefault(), move(-1));
      else if (e.key === "s") triage("saved");
      else if (e.key === "x") triage("skip");
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [move, triage]);

  const empty = !isLoading && anyJobs && anyJobs.length === 0;

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="drag flex h-[52px] shrink-0 items-center gap-3 border-b border-hairline px-4" data-tauri-drag-region>
        <h1 className="text-headline font-semibold" data-tauri-drag-region>Discover</h1>
        {!empty && (<>
        <Segmented<Scope>
          value={scope}
          onChange={(s) => {
            setScope(s);
            setSelected(null);
          }}
          options={[
            { value: "inbox", label: "New" },
            { value: "shortlist", label: "Shortlist" },
            { value: "all", label: "All" },
          ]}
        />
        <div className="flex-1" data-tauri-drag-region />
        <label className="flex items-center gap-2 text-callout text-ink-2">
          Remote only
          <Toggle checked={remoteOnly} onChange={setRemoteOnly} label="Remote only" />
        </label>
        <div className="relative w-56">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-ink-3" />
          <Input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Filter by title, company, place" className="h-7 pl-7 text-callout" />
        </div>
        </>)}
        {empty && <div className="flex-1" data-tauri-drag-region />}
        <Button busy={scan.running} icon={<RefreshCw className="size-3.5" />} onClick={scan.run}>
          {scan.running && scan.progress ? `Checking ${scan.progress.done} of ${scan.progress.total}` : "Check for new jobs"}
        </Button>
      </header>

      {scan.running && scan.progress && (
        <div className="h-0.5 shrink-0 bg-fill">
          <div className="h-full bg-gradient-to-r from-indigo to-teal transition-[width] duration-300" style={{ width: `${(scan.progress.done / Math.max(1, scan.progress.total)) * 100}%` }} />
        </div>
      )}

      {empty ? (
        <div className="scroll flex-1">
          <GettingStarted onGoTo={onGoTo} onScan={scan.run} scanning={scan.running} />
        </div>
      ) : (
        <div className="flex min-h-0 flex-1">
          <div ref={listRef} className="scroll w-[372px] shrink-0 border-r border-hairline p-2">
            {jobs.length === 0 && !isLoading && (
              <div className="px-4 py-12 text-center">
                <p className="text-body font-medium">{scope === "inbox" ? "You're all caught up" : "Nothing here yet"}</p>
                <p className="mt-1 text-callout text-ink-3">{scope === "inbox" ? "New roles from your companies land here. Check again later, or follow more companies." : "Save jobs from New with S, or add one with ⌘N."}</p>
              </div>
            )}
            {jobs.map((j) => (
              <div key={j.id} data-id={j.id}>
                <JobRow job={j} selected={j.id === selected} onSelect={() => setSelected(j.id)} />
              </div>
            ))}
            {jobs.length > 0 && <p className="px-3 py-3 text-caption text-ink-3">J and K to move, S to save, X to skip</p>}
          </div>
          <div className="min-w-0 flex-1">
            {selected != null ? <JobDetail key={selected} jobId={selected} onTriage={triage} onTailor={() => onTailor(selected)} /> : <div className="flex h-full items-center justify-center text-callout text-ink-3">Select a job</div>}
          </div>
        </div>
      )}
    </div>
  );
}
