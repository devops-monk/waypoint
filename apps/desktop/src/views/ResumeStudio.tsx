import { save } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import clsx from "clsx";
import { AlertTriangle, ArrowDown, ArrowUp, Check, FileDown, Plus, RotateCcw, ScanSearch, ShieldCheck, Sparkles, Trash2, Undo2 } from "lucide-react";
import { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { AutoTextarea, Button, Field, IconButton, Input, Panel, Segmented, SectionTitle, Tag, Textarea } from "../components/ui";
import { api, errorMessage } from "../lib/api";
import { blocked, CheckBanner, IssueList, PagePreview } from "../components/docs";
import type { AtsReport, FactIssue, JobSummary, Proposal, ResumeDoc, Template } from "../lib/types";

const csv = (v: string[]) => v.join(", ");
const fromCsv = (s: string) => s.split(",").map((x) => x.trim()).filter(Boolean);

/** Review decisions for a tailoring proposal. Keys are `${role}-${bullet}`. */
interface Review {
  rejected: Set<string>;
  restored: Record<number, number[]>;
  useSummary: boolean;
}

function applyReview(p: Proposal, r: Review): ResumeDoc {
  return {
    ...p.doc,
    summary: r.useSummary && p.summary_proposed && !blocked(p.summary_issues) ? p.summary_proposed : p.summary_original,
    experience: p.doc.experience.map((role, ri) => {
      const change = p.roles[ri];
      if (!change) return role;
      const bullets = change.bullets.map((b, bi) => (blocked(b.issues) || r.rejected.has(`${ri}-${bi}`) ? b.original : b.proposed));
      const restored = (r.restored[ri] ?? []).map((src) => change.omitted.find(([i]) => i === src)?.[1]).filter((x): x is string => !!x);
      return { ...role, bullets: [...bullets, ...restored] };
    }),
  };
}

function ReviewPanel({ proposal, review, setReview, onApply, onDiscard }: { proposal: Proposal; review: Review; setReview: (r: Review) => void; onApply: () => void; onDiscard: () => void }) {
  const toggle = (key: string) => {
    const rejected = new Set(review.rejected);
    if (rejected.has(key)) rejected.delete(key);
    else rejected.add(key);
    setReview({ ...review, rejected });
  };
  const changed = proposal.roles.reduce((n, r) => n + r.bullets.filter((b) => b.original !== b.proposed).length, 0);

  return (
    <div className="flex flex-col gap-5">
      <div className="rounded-[10px] bg-indigo/8 p-3.5">
        <p className="flex items-center gap-1.5 text-callout font-semibold text-indigo">
          <Sparkles className="size-3.5" />
          {changed} {changed === 1 ? "bullet" : "bullets"} reworded, reordered for this job
        </p>
        {proposal.rationale && <p className="selectable mt-1 text-callout text-ink-2">{proposal.rationale}</p>}
        <p className="mt-1.5 text-caption text-ink-3">Companies, titles and dates always come from your profile. Anything that adds a fact is held back.</p>
      </div>

      {proposal.summary_proposed && proposal.summary_proposed !== proposal.summary_original && (
        <div>
          <SectionTitle>Summary</SectionTitle>
          <Suggestion
            original={proposal.summary_original || "No summary"}
            proposed={proposal.summary_proposed}
            accepted={review.useSummary}
            issues={proposal.summary_issues}
            onToggle={() => setReview({ ...review, useSummary: !review.useSummary })}
          />
        </div>
      )}

      {proposal.roles.map((role, ri) => (
        <div key={ri}>
          <SectionTitle>
            {role.title}, {role.company}
          </SectionTitle>
          <div className="flex flex-col gap-2">
            {role.bullets.map((b, bi) => (
              <Suggestion key={bi} original={b.original} proposed={b.proposed} accepted={!review.rejected.has(`${ri}-${bi}`)} issues={b.issues} onToggle={() => toggle(`${ri}-${bi}`)} />
            ))}
            {role.omitted.map(([src, text]) => {
              const on = (review.restored[ri] ?? []).includes(src);
              return (
                <div key={src} className="flex items-start gap-2 rounded-[9px] border border-dashed border-fill-strong px-3 py-2">
                  <p className={clsx("selectable flex-1 text-callout", on ? "text-ink" : "text-ink-3 line-through decoration-ink-3/40")}>{text}</p>
                  <Button
                    size="sm"
                    variant="plain"
                    onClick={() => setReview({ ...review, restored: { ...review.restored, [ri]: on ? (review.restored[ri] ?? []).filter((x) => x !== src) : [...(review.restored[ri] ?? []), src] } })}
                  >
                    {on ? "Leave out" : "Add back"}
                  </Button>
                </div>
              );
            })}
          </div>
        </div>
      ))}

      {proposal.skill_issues.length > 0 && (
        <div>
          <SectionTitle>Skills</SectionTitle>
          <IssueList issues={proposal.skill_issues} />
        </div>
      )}

      <div className="sticky bottom-0 -mx-5 flex justify-end gap-2 border-t border-hairline bg-window/95 px-5 py-3 backdrop-blur">
        <Button variant="plain" onClick={onDiscard}>Discard suggestions</Button>
        <Button variant="primary" icon={<Check className="size-3.5" />} onClick={onApply}>Use these changes</Button>
      </div>
    </div>
  );
}

function Suggestion({ original, proposed, accepted, issues, onToggle }: { original: string; proposed: string; accepted: boolean; issues: FactIssue[]; onToggle: () => void }) {
  const isBlocked = blocked(issues);
  const same = original.trim() === proposed.trim();
  const showing = isBlocked || !accepted ? original : proposed;
  return (
    <div className={clsx("rounded-[9px] px-3 py-2.5", isBlocked ? "bg-coral/6" : "bg-surface shadow-[0_0_0_0.5px_var(--color-hairline)]")}>
      <div className="flex items-start gap-2">
        <p className="selectable flex-1 text-callout leading-relaxed text-ink">{showing}</p>
        {!same && !isBlocked && (
          <Button size="sm" variant="plain" icon={accepted ? <Undo2 className="size-3" /> : <Check className="size-3" />} onClick={onToggle}>
            {accepted ? "Use original" : "Use suggestion"}
          </Button>
        )}
      </div>
      {!same && (
        <p className="selectable mt-1 text-caption text-ink-3">
          {isBlocked ? "Held back: " : accepted ? "Was: " : "Suggested: "}
          <span className={clsx(isBlocked && "line-through")}>{isBlocked ? proposed : accepted ? original : proposed}</span>
        </p>
      )}
      <IssueList issues={issues} />
    </div>
  );
}

function Editor({ doc, setDoc, issues }: { doc: ResumeDoc; setDoc: (d: ResumeDoc) => void; issues: FactIssue[] }) {
  const issuesAt = (prefix: string) => issues.filter((i) => i.path === prefix);
  const setRole = (ri: number, bullets: string[]) => setDoc({ ...doc, experience: doc.experience.map((r, i) => (i === ri ? { ...r, bullets } : r)) });
  const [skillsText, setSkillsText] = useState(csv(doc.skills));
  const [contactText, setContactText] = useState(doc.contact.join(", "));
  useEffect(() => setSkillsText(csv(doc.skills)), [doc.skills]);
  useEffect(() => setContactText(doc.contact.join(", ")), [doc.contact]);

  return (
    <div className="flex flex-col gap-6">
      <div className="grid grid-cols-2 gap-3">
        <Field label="Headline"><Input value={doc.headline} onChange={(e) => setDoc({ ...doc, headline: e.target.value })} /></Field>
        <Field label="Contact line" hint="Separate with commas"><Input value={contactText} onChange={(e) => setContactText(e.target.value)} onBlur={() => setDoc({ ...doc, contact: fromCsv(contactText) })} /></Field>
      </div>
      <div>
        <Field label="Summary"><Textarea value={doc.summary} onChange={(e) => setDoc({ ...doc, summary: e.target.value })} className="min-h-20 text-callout" /></Field>
        <IssueList issues={issuesAt("summary")} />
      </div>
      {doc.experience.map((role, ri) => (
        <div key={ri}>
          <SectionTitle>
            {role.title}, {role.company}
            <span className="ml-2 font-normal text-ink-3">{role.start} – {role.end}</span>
          </SectionTitle>
          <IssueList issues={issues.filter((i) => i.path.startsWith(`experience.${ri}`) && !i.path.includes("bullets"))} />
          <div className="flex flex-col gap-1.5">
            {role.bullets.map((b, bi) => (
              <div key={bi}>
                <div className="group flex items-start gap-1">
                  <AutoTextarea value={b} onChange={(e) => setRole(ri, role.bullets.map((x, i) => (i === bi ? e.target.value : x)))} className="flex-1 text-callout" />
                  <div className="flex pt-1 opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100">
                    <IconButton label="Move up" className="size-6" disabled={bi === 0} onClick={() => { const n = [...role.bullets]; [n[bi - 1], n[bi]] = [n[bi], n[bi - 1]]; setRole(ri, n); }}><ArrowUp className="size-3" /></IconButton>
                    <IconButton label="Move down" className="size-6" disabled={bi === role.bullets.length - 1} onClick={() => { const n = [...role.bullets]; [n[bi + 1], n[bi]] = [n[bi], n[bi + 1]]; setRole(ri, n); }}><ArrowDown className="size-3" /></IconButton>
                    <IconButton label="Remove bullet" className="size-6" onClick={() => setRole(ri, role.bullets.filter((_, i) => i !== bi))}><Trash2 className="size-3" /></IconButton>
                  </div>
                </div>
                <IssueList issues={issuesAt(`experience.${ri}.bullets.${bi}`)} />
              </div>
            ))}
            <Button size="sm" variant="plain" className="self-start" icon={<Plus className="size-3" />} onClick={() => setRole(ri, [...role.bullets, ""])}>Add bullet</Button>
          </div>
        </div>
      ))}
      <div>
        <Field label="Skills" hint="Separate with commas. Only skills in your profile pass the fact check.">
          <Input value={skillsText} onChange={(e) => setSkillsText(e.target.value)} onBlur={() => setDoc({ ...doc, skills: fromCsv(skillsText) })} />
        </Field>
        <IssueList issues={issues.filter((i) => i.path.startsWith("skills"))} />
      </div>
      <p className="text-caption text-ink-3">To change a company, title or dates, edit your Profile. The resume always uses those exactly.</p>
    </div>
  );
}

function AtsPanel({ report }: { report: AtsReport }) {
  const pct = Math.round(report.coverage * 100);
  const missing = report.keywords.filter((k) => !k.found);
  return (
    <Panel className="p-4">
      <div className="flex items-center gap-3">
        <ScanSearch className="size-4 text-ink-2" />
        <p className="flex-1 text-callout font-semibold">How a tracking system reads it</p>
        <span className="tabular text-callout text-ink-2">{report.pages} {report.pages === 1 ? "page" : "pages"}</span>
      </div>
      {report.problems.length === 0 ? (
        <p className="mt-2 flex items-center gap-1.5 text-callout text-teal"><ShieldCheck className="size-3.5" />Name, sections and roles all read back cleanly.</p>
      ) : (
        <ul className="mt-2 flex flex-col gap-1">{report.problems.map((p) => <li key={p} className="flex gap-1.5 text-callout text-amber"><AlertTriangle className="mt-0.5 size-3.5 shrink-0" />{p}</li>)}</ul>
      )}
      {report.keywords.length > 0 && (
        <div className="mt-3">
          <div className="flex items-baseline justify-between text-callout">
            <span className="font-medium">Posting keywords found</span>
            <span className="tabular text-ink-2">{pct}%</span>
          </div>
          <div className="mt-1 h-1.5 overflow-hidden rounded-full bg-fill-strong">
            <div className="h-full rounded-full bg-gradient-to-r from-indigo to-teal" style={{ width: `${pct}%` }} />
          </div>
          <div className="mt-2 flex flex-wrap gap-1">
            {report.keywords.map((k) => <Tag key={k.keyword} tone={k.found ? "teal" : "neutral"}>{k.keyword}</Tag>)}
          </div>
          {missing.length > 0 && <p className="mt-2 text-caption text-ink-3">Add a missing keyword only where your experience genuinely covers it, ideally in your Profile so every resume benefits.</p>}
        </div>
      )}
    </Panel>
  );
}

export function ResumeStudio({ jobId, job, leading }: { jobId: number | null; job?: JobSummary; leading: ReactNode }) {
  const [doc, setDoc] = useState<ResumeDoc | null>(null);
  const [template, setTemplate] = useState<Template>("clean");
  const [savedAt, setSavedAt] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);
  const [proposal, setProposal] = useState<Proposal | null>(null);
  const [review, setReview] = useState<Review>({ rejected: new Set(), restored: {}, useSummary: true });
  const [pages, setPages] = useState<string[]>([]);
  const [issues, setIssues] = useState<FactIssue[]>([]);
  const [renderError, setRenderError] = useState<string | null>(null);
  const [ats, setAts] = useState<AtsReport | null>(null);
  const [busy, setBusy] = useState<"tailor" | "export" | "word" | "ats" | "save" | null>(null);
  const renderSeq = useRef(0);

  useEffect(() => {
    let live = true;
    setProposal(null);
    setAts(null);
    api.resumeLoad(jobId).then((s) => {
      if (!live) return;
      setDoc(s.doc);
      setTemplate(s.template);
      setSavedAt(s.saved_at);
      setDirty(false);
    }).catch((e) => toast.error(errorMessage(e)));
    return () => {
      live = false;
    };
  }, [jobId]);

  const shown = useMemo(() => (proposal ? applyReview(proposal, review) : doc), [proposal, review, doc]);

  // Live preview, debounced; stale responses are dropped.
  useEffect(() => {
    if (!shown) return;
    const seq = ++renderSeq.current;
    const t = setTimeout(() => {
      api.resumeRender(shown, template)
        .then((r) => {
          if (seq !== renderSeq.current) return;
          setPages(r.pages);
          setIssues(r.issues);
          setRenderError(null);
        })
        .catch((e) => seq === renderSeq.current && setRenderError(errorMessage(e)));
    }, 220);
    return () => clearTimeout(t);
  }, [shown, template]);

  const edit = (d: ResumeDoc) => {
    setDoc(d);
    setDirty(true);
    setAts(null);
  };

  const tailor = async () => {
    if (jobId == null) return;
    setBusy("tailor");
    try {
      const p = await api.resumeTailor(jobId);
      setProposal(p);
      setReview({ rejected: new Set(), restored: {}, useSummary: true });
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
    }
  };

  const saveVersion = async () => {
    if (!doc) return;
    setBusy("save");
    try {
      await api.resumeSave(jobId, doc, template);
      setSavedAt(new Date().toISOString());
      setDirty(false);
      toast.success(jobId ? `Saved resume for ${job?.company ?? "this job"}` : "Saved your base resume");
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
    }
  };

  const runAts = async () => {
    if (!shown) return;
    setBusy("ats");
    try {
      setAts(await api.resumeAts(jobId, shown, template));
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
    }
  };

  const exportFile = async (format: "pdf" | "docx") => {
    if (!doc) return;
    const base = [doc.name || "Resume", job?.company].filter(Boolean).join(" - ");
    const filter = format === "pdf" ? { name: "PDF", extensions: ["pdf"] } : { name: "Word document", extensions: ["docx"] };
    const path = await save({ defaultPath: `${base}.${format}`, filters: [filter] });
    if (!path) return;
    setBusy(format === "pdf" ? "export" : "word");
    try {
      const r = await api.resumeExport(jobId, doc, template, path, format);
      setAts(r.ats);
      setSavedAt(new Date().toISOString());
      setDirty(false);
      toast.success(format === "pdf" ? "Resume exported as PDF" : "Resume exported as a Word file", { action: { label: "Show file", onClick: () => revealItemInDir(r.path) } });
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
    }
  };

  const blocks = issues.filter((i) => i.severity === "block").length;

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="drag flex h-[52px] shrink-0 items-center gap-3 border-b border-hairline px-4" data-tauri-drag-region>
        {leading}
        <div className="flex-1" data-tauri-drag-region />
        <Segmented<Template> value={template} onChange={(t) => { setTemplate(t); setDirty(true); }} options={[{ value: "clean", label: "Clean" }, { value: "modern", label: "Modern" }]} />
        {doc && (
          <Segmented value={doc.paper === "us-letter" ? "us-letter" : "a4"} onChange={(p) => edit({ ...doc, paper: p })} options={[{ value: "a4", label: "A4" }, { value: "us-letter", label: "Letter" }]} />
        )}
        {jobId != null && !proposal && (
          <Button busy={busy === "tailor"} icon={<Sparkles className="size-3.5" />} onClick={tailor}>
            {busy === "tailor" ? "Tailoring…" : "Tailor for this job"}
          </Button>
        )}
        <Button variant="plain" busy={busy === "save"} disabled={!dirty || !!proposal} onClick={saveVersion}>Save</Button>
        <Button busy={busy === "word"} disabled={!!proposal || blocks > 0} onClick={() => exportFile("docx")} title={blocks > 0 ? "Fix the fact-check issues first" : "Export as a Word document"}>
          Word
        </Button>
        <Button variant="primary" busy={busy === "export"} disabled={!!proposal || blocks > 0} icon={<FileDown className="size-3.5" />} onClick={() => exportFile("pdf")} title={blocks > 0 ? "Fix the fact-check issues first" : undefined}>
          Export PDF
        </Button>
      </header>

      <div className="flex min-h-0 flex-1">
        <div className="scroll w-[460px] shrink-0 border-r border-hairline px-5 pt-5">
          {!doc ? null : doc.experience.length === 0 && !doc.summary ? (
            <div className="py-12 text-center">
              <p className="text-body font-medium">Your resume starts from your profile</p>
              <p className="mt-1 text-callout text-ink-3">Add your experience in Profile, then come back to tailor it for each job.</p>
            </div>
          ) : (
            <>
              <CheckBanner issues={issues} clean="Every fact traces back to your profile." />
              {proposal ? (
                <ReviewPanel
                  proposal={proposal}
                  review={review}
                  setReview={setReview}
                  onDiscard={() => setProposal(null)}
                  onApply={() => {
                    edit(applyReview(proposal, review));
                    setProposal(null);
                    toast.success("Changes applied. Save or export when you're ready.");
                  }}
                />
              ) : (
                <>
                  <Editor doc={doc} setDoc={edit} issues={issues} />
                  <div className="my-6 flex flex-col gap-3">
                    {ats ? <AtsPanel report={ats} /> : null}
                    <Button variant="secondary" className="self-start" busy={busy === "ats"} icon={<ScanSearch className="size-3.5" />} onClick={runAts}>
                      {ats ? "Check again" : "Check how tracking systems read it"}
                    </Button>
                    {savedAt && !dirty && <p className="flex items-center gap-1.5 text-caption text-ink-3"><RotateCcw className="size-3" />Saved {new Date(savedAt).toLocaleString()}</p>}
                  </div>
                </>
              )}
            </>
          )}
        </div>

        <PagePreview pages={pages} error={renderError} />
      </div>
    </div>
  );
}
