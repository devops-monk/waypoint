import { listen } from "@tauri-apps/api/event";
import { save } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import clsx from "clsx";
import { AlertTriangle, ArrowDown, ArrowUp, Check, ChevronRight, CircleDashed, FileDown, GitCompareArrows, Plus, RotateCcw, Sparkles, Trash2 } from "lucide-react";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { CheckBanner, IssueList, PagePreview } from "../components/docs";
import { AutoTextarea, Button, Field, IconButton, Input, Panel, Segmented, SectionTitle, Tag } from "../components/ui";
import { api, errorMessage } from "../lib/api";
import type { FactIssue, JobSummary, LetterDoc, LetterNotes, Template } from "../lib/types";

type Stage = "drafting" | "reviewing" | "revising";

const PASSES: { stage: Stage; title: string; running: string; detail: string }[] = [
  { stage: "drafting", title: "Draft", running: "Drafting", detail: "A first draft that answers every requirement in the posting, using only your profile." },
  { stage: "reviewing", title: "Independent review", running: "Reviewing", detail: "A separate reviewer reads it as the hiring manager would and checks every claim against your profile." },
  { stage: "revising", title: "Revision", running: "Revising", detail: "The draft is rewritten from the review. You'll see what changed and what was turned down." },
];

/** The three passes as stops on a route, the app's signature motif. */
function WritingRoute({ stage }: { stage: Stage | null }) {
  const at = stage ? PASSES.findIndex((p) => p.stage === stage) : -1;
  return (
    <ol className="flex flex-col">
      {PASSES.map((p, i) => {
        const done = i < at;
        const current = i === at;
        return (
          <li key={p.stage} className="relative flex gap-3 pb-5 last:pb-0">
            {i < PASSES.length - 1 && (
              <span aria-hidden className={clsx("absolute top-4 bottom-0 left-[5px] border-l-2 border-dotted transition-colors", done ? "border-teal" : "border-fill-strong")} />
            )}
            <span className="flex h-4 w-3 shrink-0 items-center justify-center">
              <span
                className={clsx(
                  "block rounded-full transition-all duration-300 ease-spring",
                  current ? "size-3 animate-pulse bg-teal ring-4 ring-teal/20" : done ? "size-2.5 bg-teal" : "size-2.5 bg-fill-strong",
                )}
              />
            </span>
            <div className="min-w-0">
              <p className={clsx("text-callout font-semibold", current ? "text-teal" : "text-ink")}>
                {current ? `${p.running}…` : p.title}
                {done && <Check className="ml-1 inline size-3.5 text-teal" />}
              </p>
              <p className="mt-0.5 text-callout text-ink-2">{p.detail}</p>
            </div>
          </li>
        );
      })}
    </ol>
  );
}

const VERDICT: Record<string, { label: string; tone: "teal" | "amber" | "coral" }> = {
  ready: { label: "Ready", tone: "teal" },
  minor_fixes: { label: "Minor fixes", tone: "amber" },
  needs_work: { label: "Needs work", tone: "coral" },
};

const AREA: Record<string, string> = {
  grounding: "Facts",
  requirements: "Requirements",
  company: "Company",
  tone: "Tone",
  structure: "Structure",
  length: "Length",
};

function ReviewNotes({ notes }: { notes: LetterNotes }) {
  const [open, setOpen] = useState(false);
  const v = VERDICT[notes.review.verdict] ?? { label: notes.review.verdict, tone: "amber" as const };
  return (
    <Panel className="p-4">
      <div className="flex items-center gap-2">
        <GitCompareArrows className="size-4 text-ink-2" />
        <p className="flex-1 text-callout font-semibold">What the reviewer said</p>
        <span className="text-caption text-ink-3">First draft</span>
        <Tag tone={v.tone}>{v.label}</Tag>
      </div>
      {notes.review.overall && <p className="selectable mt-2 text-callout text-ink-2">{notes.review.overall}</p>}
      {notes.changes.length > 0 && (
        <div className="mt-3">
          <p className="text-caption font-semibold text-ink-2">Changed in the revision</p>
          <ul className="mt-1 flex flex-col gap-1">
            {notes.changes.map((c, i) => (
              <li key={i} className="selectable flex gap-1.5 text-callout text-ink"><Check className="mt-0.5 size-3.5 shrink-0 text-teal" />{c}</li>
            ))}
          </ul>
        </div>
      )}
      {notes.declined.length > 0 && (
        <div className="mt-3">
          <p className="text-caption font-semibold text-ink-2">Not applied</p>
          <ul className="mt-1 flex flex-col gap-1">
            {notes.declined.map((c, i) => (
              <li key={i} className="selectable flex gap-1.5 text-callout text-ink-2"><CircleDashed className="mt-0.5 size-3.5 shrink-0 text-ink-3" />{c}</li>
            ))}
          </ul>
        </div>
      )}
      {notes.review.findings.length > 0 && (
        <>
          <button onClick={() => setOpen(!open)} className="mt-3 flex items-center gap-1 text-callout font-medium text-indigo hover:underline" aria-expanded={open}>
            <ChevronRight className={clsx("size-3.5 transition-transform", open && "rotate-90")} />
            {open ? "Hide" : "Show"} all {notes.review.findings.length} review notes
          </button>
          {open && (
            <ul className="mt-2 flex flex-col gap-2.5">
              {notes.review.findings.map((f, i) => (
                <li key={i} className="rounded-[9px] bg-fill px-3 py-2">
                  <Tag tone={f.area === "grounding" ? "coral" : "neutral"}>{AREA[f.area] ?? f.area}</Tag>
                  {f.quote && <p className="selectable mt-1.5 text-callout text-ink-2 italic">“{f.quote}”</p>}
                  <p className="selectable mt-1 text-callout text-ink">{f.problem}</p>
                  {f.suggestion && <p className="selectable mt-0.5 text-callout text-ink-2">{f.suggestion}</p>}
                </li>
              ))}
            </ul>
          )}
        </>
      )}
      <p className="mt-3 text-caption text-ink-3">Written {new Date(notes.written_at).toLocaleString()} with {notes.model}</p>
    </Panel>
  );
}

const REQ: Record<string, { label: string; tone: "teal" | "amber" | "coral"; dot: string }> = {
  matched: { label: "matched", tone: "teal", dot: "bg-teal" },
  bridged: { label: "bridged", tone: "amber", dot: "bg-amber" },
  gap: { label: "gap", tone: "coral", dot: "bg-coral" },
};

function Requirements({ notes }: { notes: LetterNotes }) {
  if (!notes.requirements.length) return null;
  const count = (s: string) => notes.requirements.filter((r) => r.status === s).length;
  return (
    <Panel className="p-4">
      <div className="flex items-center gap-1.5">
        <p className="flex-1 text-callout font-semibold">What the posting asks for</p>
        {(["matched", "bridged", "gap"] as const).map((s) => count(s) > 0 && <Tag key={s} tone={REQ[s].tone}>{count(s)} {REQ[s].label}</Tag>)}
      </div>
      <ul className="mt-3 flex flex-col gap-2">
        {notes.requirements.map((r, i) => (
          <li key={i} className="flex gap-2.5">
            <span className={clsx("mt-1.5 size-2 shrink-0 rounded-full", REQ[r.status]?.dot ?? "bg-fill-strong")} title={REQ[r.status]?.label} />
            <div className="min-w-0">
              <p className="selectable text-callout text-ink">{r.requirement}</p>
              {r.evidence && <p className="selectable text-caption text-ink-3">{r.evidence}</p>}
            </div>
          </li>
        ))}
      </ul>
      <p className="mt-3 text-caption text-ink-3">Bridged means the letter names the gap and connects it to something you have done. Gaps are left honest rather than hidden.</p>
    </Panel>
  );
}

function ListEditor({ items, onChange, issues, path, addLabel }: { items: string[]; onChange: (v: string[]) => void; issues: FactIssue[]; path: string; addLabel: string }) {
  return (
    <div className="flex flex-col gap-1.5">
      {items.map((t, i) => (
        <div key={i}>
          <div className="group flex items-start gap-1">
            <AutoTextarea value={t} onChange={(e) => onChange(items.map((x, k) => (k === i ? e.target.value : x)))} className="flex-1 text-callout" />
            <div className="flex pt-1 opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100">
              <IconButton label="Move up" className="size-6" disabled={i === 0} onClick={() => { const n = [...items]; [n[i - 1], n[i]] = [n[i], n[i - 1]]; onChange(n); }}><ArrowUp className="size-3" /></IconButton>
              <IconButton label="Move down" className="size-6" disabled={i === items.length - 1} onClick={() => { const n = [...items]; [n[i + 1], n[i]] = [n[i], n[i + 1]]; onChange(n); }}><ArrowDown className="size-3" /></IconButton>
              <IconButton label="Remove" className="size-6" onClick={() => onChange(items.filter((_, k) => k !== i))}><Trash2 className="size-3" /></IconButton>
            </div>
          </div>
          <IssueList issues={issues.filter((x) => x.path === `${path}.${i}`)} />
        </div>
      ))}
      <Button size="sm" variant="plain" className="self-start" icon={<Plus className="size-3" />} onClick={() => onChange([...items, ""])}>{addLabel}</Button>
    </div>
  );
}

function Editor({ doc, setDoc, issues }: { doc: LetterDoc; setDoc: (d: LetterDoc) => void; issues: FactIssue[] }) {
  const at = (p: string) => issues.filter((i) => i.path === p);
  return (
    <div className="flex flex-col gap-5">
      <div className="grid grid-cols-[1fr_150px] gap-3">
        <Field label="To" hint="One line each">
          <AutoTextarea value={doc.recipient.join("\n")} onChange={(e) => setDoc({ ...doc, recipient: e.target.value.split("\n") })} className="text-callout" />
        </Field>
        <Field label="Date"><Input value={doc.date} onChange={(e) => setDoc({ ...doc, date: e.target.value })} /></Field>
      </div>
      <div>
        <Field label="Subject" hint="Specific to you and this role, not “Application for…”"><Input value={doc.subject} onChange={(e) => setDoc({ ...doc, subject: e.target.value })} /></Field>
        <IssueList issues={at("subject")} />
      </div>
      <div>
        <Field label="Greeting"><Input value={doc.salutation} onChange={(e) => setDoc({ ...doc, salutation: e.target.value })} /></Field>
        <IssueList issues={at("salutation")} />
      </div>
      <div>
        <Field label="Opening"><AutoTextarea value={doc.opening} onChange={(e) => setDoc({ ...doc, opening: e.target.value })} className="text-callout" /></Field>
        <IssueList issues={at("opening")} />
      </div>
      <div>
        <SectionTitle>Highlights</SectionTitle>
        <ListEditor items={doc.highlights} onChange={(highlights) => setDoc({ ...doc, highlights })} issues={issues} path="highlights" addLabel="Add highlight" />
      </div>
      <div>
        <SectionTitle>Paragraphs</SectionTitle>
        <ListEditor items={doc.body} onChange={(body) => setDoc({ ...doc, body })} issues={issues} path="body" addLabel="Add paragraph" />
      </div>
      <div className="grid grid-cols-2 gap-3">
        <Field label="Sign-off"><Input value={doc.closing} onChange={(e) => setDoc({ ...doc, closing: e.target.value })} /></Field>
        <Field label="Name"><Input value={doc.signature} onChange={(e) => setDoc({ ...doc, signature: e.target.value })} /></Field>
      </div>
      <p className="text-caption text-ink-3">Your name and contact line come from your resume, so the two documents match.</p>
    </div>
  );
}

export function CoverLetterStudio({ jobId, job, leading }: { jobId: number | null; job?: JobSummary; leading: ReactNode }) {
  const [doc, setDoc] = useState<LetterDoc | null>(null);
  const [template, setTemplate] = useState<Template>("clean");
  const [notes, setNotes] = useState<LetterNotes | null>(null);
  const [fitWarning, setFitWarning] = useState<string | null>(null);
  const [savedAt, setSavedAt] = useState<string | null>(null);
  const [started, setStarted] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [pages, setPages] = useState<string[]>([]);
  const [issues, setIssues] = useState<FactIssue[]>([]);
  const [words, setWords] = useState(0);
  const [renderError, setRenderError] = useState<string | null>(null);
  const [busy, setBusy] = useState<"write" | "save" | "export" | "word" | null>(null);
  const [stage, setStage] = useState<Stage | null>(null);
  const renderSeq = useRef(0);

  useEffect(() => {
    if (jobId == null) return;
    let live = true;
    setDoc(null);
    setPages([]);
    api.letterLoad(jobId).then((s) => {
      if (!live) return;
      setDoc(s.doc);
      setTemplate(s.template);
      setNotes(s.notes);
      setFitWarning(s.fit_warning);
      setSavedAt(s.saved_at);
      setStarted(s.saved_at != null);
      setDirty(false);
    }).catch((e) => toast.error(errorMessage(e)));
    return () => {
      live = false;
    };
  }, [jobId]);

  useEffect(() => {
    const off = listen<{ job_id: number; stage: Stage }>("letter://progress", (e) => {
      if (e.payload.job_id === jobId) setStage(e.payload.stage);
    });
    return () => {
      off.then((f) => f());
    };
  }, [jobId]);

  useEffect(() => {
    if (!doc || jobId == null) return;
    const seq = ++renderSeq.current;
    const t = setTimeout(() => {
      api.letterRender(jobId, doc, template)
        .then((r) => {
          if (seq !== renderSeq.current) return;
          setPages(r.pages);
          setIssues(r.issues);
          setWords(r.words);
          setRenderError(null);
        })
        .catch((e) => seq === renderSeq.current && setRenderError(errorMessage(e)));
    }, 220);
    return () => clearTimeout(t);
  }, [doc, template, jobId]);

  if (jobId == null) {
    return (
      <div className="flex min-h-0 flex-1 flex-col">
        <header className="drag flex h-[52px] shrink-0 items-center gap-3 border-b border-hairline px-4" data-tauri-drag-region>{leading}</header>
        <div className="flex flex-1 items-center justify-center p-8">
          <div className="max-w-sm text-center">
            <p className="text-body font-medium">Cover letters are written for one job</p>
            <p className="mt-1 text-callout text-ink-3">Choose a job above, or open one in Discover and pick Cover letter.</p>
          </div>
        </div>
      </div>
    );
  }

  const edit = (d: LetterDoc) => {
    setDoc(d);
    setDirty(true);
  };

  const write = async () => {
    setBusy("write");
    setStage("drafting");
    try {
      if (dirty && doc) {
        await api.letterSave(jobId, doc, template);
        toast("Your edits were saved as a version first.");
      }
      const s = await api.letterWrite(jobId);
      setDoc(s.doc);
      setTemplate(s.template);
      setNotes(s.notes);
      setSavedAt(s.saved_at);
      setStarted(true);
      setDirty(false);
      toast.success("Cover letter written and saved");
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
      setStage(null);
    }
  };

  const saveVersion = async () => {
    if (!doc) return;
    setBusy("save");
    try {
      await api.letterSave(jobId, doc, template);
      setSavedAt(new Date().toISOString());
      setDirty(false);
      toast.success(`Saved cover letter for ${job?.company ?? "this job"}`);
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
    }
  };

  const exportFile = async (format: "pdf" | "docx") => {
    if (!doc) return;
    const base = [doc.name || "Cover letter", "Cover letter", job?.company].filter(Boolean).join(" - ");
    const filter = format === "pdf" ? { name: "PDF", extensions: ["pdf"] } : { name: "Word document", extensions: ["docx"] };
    const path = await save({ defaultPath: `${base}.${format}`, filters: [filter] });
    if (!path) return;
    setBusy(format === "pdf" ? "export" : "word");
    try {
      const out = await api.letterExport(jobId, doc, template, path, format);
      setSavedAt(new Date().toISOString());
      setDirty(false);
      toast.success(format === "pdf" ? "Cover letter exported as PDF" : "Cover letter exported as a Word file", { action: { label: "Show file", onClick: () => revealItemInDir(out) } });
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
    }
  };

  const blocks = issues.filter((i) => i.severity === "block").length;
  const writing = busy === "write";

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="drag flex h-[52px] shrink-0 items-center gap-3 border-b border-hairline px-4" data-tauri-drag-region>
        {leading}
        <div className="flex-1" data-tauri-drag-region />
        <Segmented<Template> value={template} onChange={(t) => { setTemplate(t); setDirty(true); }} options={[{ value: "clean", label: "Clean" }, { value: "modern", label: "Modern" }]} />
        {doc && <Segmented value={doc.paper === "us-letter" ? "us-letter" : "a4"} onChange={(p) => edit({ ...doc, paper: p })} options={[{ value: "a4", label: "A4" }, { value: "us-letter", label: "Letter" }]} />}
        {started && (
          <Button busy={writing} icon={<Sparkles className="size-3.5" />} onClick={write}>
            {writing ? "Writing…" : "Write again"}
          </Button>
        )}
        <Button variant="plain" busy={busy === "save"} disabled={!dirty || !started} onClick={saveVersion}>Save</Button>
        <Button busy={busy === "word"} disabled={!started || blocks > 0 || writing} onClick={() => exportFile("docx")} title={blocks > 0 ? "Fix the fact-check issues first" : "Export as a Word document"}>
          Word
        </Button>
        <Button variant="primary" busy={busy === "export"} disabled={!started || blocks > 0 || writing} icon={<FileDown className="size-3.5" />} onClick={() => exportFile("pdf")} title={blocks > 0 ? "Fix the fact-check issues first" : undefined}>
          Export PDF
        </Button>
      </header>

      <div className="flex min-h-0 flex-1">
        <div className="scroll w-[460px] shrink-0 border-r border-hairline px-5 pt-5">
          {!doc ? null : !started || writing ? (
            <div className="pb-8">
              <h2 className="text-headline font-semibold">{writing ? "Writing your cover letter" : "Cover letter"}</h2>
              <p className="mt-1 text-callout text-ink-2">
                {job ? <>For {job.title} at {job.company}. </> : null}
                Written in three passes, using only facts from your profile.
              </p>
              {fitWarning && (
                <p className="mt-4 flex gap-2 rounded-[9px] bg-amber/10 px-3 py-2 text-callout text-amber">
                  <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
                  {fitWarning}
                </p>
              )}
              <div className="mt-6">
                <WritingRoute stage={writing ? stage : null} />
              </div>
              {writing ? (
                <p className="mt-6 text-caption text-ink-3">This takes a minute or two. You can keep using the app.</p>
              ) : (
                <div className="mt-7 flex items-center gap-2">
                  <Button variant="primary" icon={<Sparkles className="size-3.5" />} onClick={write}>Write cover letter</Button>
                  <Button variant="plain" onClick={() => setStarted(true)}>Start from a blank letter</Button>
                </div>
              )}
            </div>
          ) : (
            <>
              <CheckBanner issues={issues} clean="Every claim traces back to your profile or the posting." extra={`${words} words`} />
              {fitWarning && !notes && (
                <p className="mb-5 flex gap-2 rounded-[9px] bg-amber/10 px-3 py-2 text-callout text-amber"><AlertTriangle className="mt-0.5 size-3.5 shrink-0" />{fitWarning}</p>
              )}
              {notes && (
                <div className="mb-6 flex flex-col gap-3">
                  <ReviewNotes notes={notes} />
                  <Requirements notes={notes} />
                </div>
              )}
              <IssueList issues={issues.filter((i) => i.path === "length")} />
              <div className="mt-3">
                <Editor doc={doc} setDoc={edit} issues={issues} />
              </div>
              <div className="my-6">
                {savedAt && !dirty && <p className="flex items-center gap-1.5 text-caption text-ink-3"><RotateCcw className="size-3" />Saved {new Date(savedAt).toLocaleString()}</p>}
              </div>
            </>
          )}
        </div>
        <PagePreview pages={pages} error={renderError} />
      </div>
    </div>
  );
}
