import { useQuery, useQueryClient } from "@tanstack/react-query";
import clsx from "clsx";
import { BookOpen, Check, ChevronRight, MessageSquareQuote, Plus, RefreshCw, Sparkles, Trash2 } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { JobPicker } from "../components/docs";
import { AutoTextarea, Button, Field, Input, Panel, Segmented, SectionTitle, Tag } from "../components/ui";
import { api, errorMessage } from "../lib/api";
import type { Feedback, JobSummary, Practice, PrepPack, PrepState, QuestionCategory, Story } from "../lib/types";

const CATEGORY: Record<QuestionCategory, string> = {
  opener: "Opening",
  motivation: "Why this role",
  behavioral: "Behavioral",
  technical: "Role and technical",
  curveball: "Curveballs",
};
const ORDER: QuestionCategory[] = ["opener", "motivation", "behavioral", "technical", "curveball"];
const STAR: { key: keyof Feedback["star"]; label: string }[] = [
  { key: "situation", label: "Situation" },
  { key: "task", label: "Task" },
  { key: "action", label: "Action" },
  { key: "result", label: "Result" },
];

function StarText({ story }: { story: Story }) {
  return (
    <dl className="mt-2 grid grid-cols-[18px_1fr] gap-x-2 gap-y-1.5 rounded-[9px] bg-fill px-3 py-2.5 text-callout">
      {(["situation", "task", "action", "result"] as const).map((k) => (
        <div key={k} className="contents">
          <dt className="font-semibold text-teal">{k[0].toUpperCase()}</dt>
          <dd className="selectable m-0 text-ink">{story[k] || <span className="text-ink-3">Not written yet</span>}</dd>
        </div>
      ))}
    </dl>
  );
}

function FeedbackView({ f }: { f: Feedback }) {
  const [show, setShow] = useState(false);
  return (
    <div className="mt-3 rounded-[10px] bg-indigo/6 p-3">
      <div className="flex flex-wrap items-center gap-2">
        <span className="flex gap-0.5" aria-label={`${f.score} out of 5`}>
          {[1, 2, 3, 4, 5].map((i) => <span key={i} className={clsx("size-2 rounded-full", i <= f.score ? "bg-indigo" : "bg-fill-strong")} />)}
        </span>
        <p className="flex-1 text-callout font-medium">{f.summary}</p>
      </div>
      <div className="mt-2 flex flex-wrap gap-1">
        {STAR.map((s) => <Tag key={s.key} tone={f.star[s.key] ? "teal" : "amber"}>{f.star[s.key] ? "✓" : "Missing:"} {s.label}</Tag>)}
      </div>
      <div className="mt-2.5 grid grid-cols-2 gap-4 text-callout">
        <div>
          <p className="text-caption font-semibold text-ink-2">What worked</p>
          <ul className="mt-1 flex flex-col gap-1">{f.worked.map((w, i) => <li key={i} className="selectable">{w}</li>)}</ul>
        </div>
        <div>
          <p className="text-caption font-semibold text-ink-2">To sharpen</p>
          <ul className="mt-1 flex flex-col gap-1">{f.improve.map((w, i) => <li key={i} className="selectable">{w}</li>)}</ul>
        </div>
      </div>
      <button className="mt-2.5 flex items-center gap-1 text-callout font-medium text-indigo hover:underline" onClick={() => setShow(!show)} aria-expanded={show}>
        <ChevronRight className={clsx("size-3.5 transition-transform", show && "rotate-90")} />
        {show ? "Hide" : "Show"} a stronger version
      </button>
      {show && <p className="selectable mt-1.5 whitespace-pre-wrap text-callout leading-relaxed text-ink">{f.stronger_answer}</p>}
      <p className="mt-2 text-caption text-ink-3">The stronger version uses only your own facts. Fill in anything in [brackets] before you rely on it.</p>
    </div>
  );
}

function QuestionCard({ q, story, practice, jobId, onPracticed }: { q: PrepPack["questions"][number]; story?: Story; practice?: Practice; jobId: number; onPracticed: () => void }) {
  const [open, setOpen] = useState(false);
  const [showStory, setShowStory] = useState(false);
  const [answer, setAnswer] = useState("");
  const [busy, setBusy] = useState(false);
  const submit = async () => {
    setBusy(true);
    try {
      await api.practiceAnswer(jobId, q.question, answer);
      setAnswer("");
      onPracticed();
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <li className="rounded-[12px] bg-surface p-4 shadow-[0_0_0_0.5px_var(--color-hairline),0_1px_2px_rgb(0_0_0/0.04)]">
      <p className="selectable text-body font-semibold">{q.question}</p>
      <p className="selectable mt-0.5 text-callout text-ink-3">{q.why}</p>
      {q.talking_points.length > 0 && (
        <ul className="mt-2.5 flex flex-col gap-1">
          {q.talking_points.map((t, i) => (
            <li key={i} className="selectable flex gap-2 text-callout text-ink"><span className="mt-[7px] size-1.5 shrink-0 rounded-full bg-teal" />{t}</li>
          ))}
        </ul>
      )}
      <div className="mt-3 flex flex-wrap items-center gap-2">
        {story && (
          <Button size="sm" variant="plain" icon={<BookOpen className="size-3" />} onClick={() => setShowStory(!showStory)}>
            Tell: {story.title}
          </Button>
        )}
        <div className="flex-1" />
        {practice && !open && <span className="text-caption text-ink-3">Practiced, {practice.feedback.score}/5</span>}
        <Button size="sm" icon={<MessageSquareQuote className="size-3" />} onClick={() => setOpen(!open)}>{open ? "Close practice" : "Practice"}</Button>
      </div>
      {showStory && story && <StarText story={story} />}
      {open && (
        <div className="mt-3">
          <AutoTextarea value={answer} onChange={(e) => setAnswer(e.target.value)} placeholder="Answer the way you'd say it out loud." className="min-h-24 text-callout" />
          <div className="mt-2 flex items-center gap-3">
            <span className="tabular text-caption text-ink-3">{answer.split(/\s+/).filter(Boolean).length} words</span>
            <div className="flex-1" />
            <Button size="sm" variant="primary" busy={busy} disabled={!answer.trim()} onClick={submit}>Get feedback</Button>
          </div>
          {practice && <FeedbackView f={practice.feedback} />}
        </div>
      )}
    </li>
  );
}

function PrepView({ jobId, job }: { jobId: number; job?: JobSummary }) {
  const qc = useQueryClient();
  const { data: prep } = useQuery({ queryKey: ["prep", jobId], queryFn: () => api.prepGet(jobId) });
  const { data: stories = [] } = useQuery({ queryKey: ["stories"], queryFn: api.storiesList });
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<string[]>([]);
  const [debrief, setDebrief] = useState("");
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    setDone(prep?.done ?? []);
    setDebrief(prep?.debrief ?? "");
  }, [prep?.done, prep?.debrief]);

  const persist = (d: string[], text: string) => {
    if (saveTimer.current) clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(() => api.prepNotes(jobId, d, text).catch((e) => toast.error(errorMessage(e))), 600);
  };

  const build = async () => {
    setBusy(true);
    try {
      const s = await api.prepBuild(jobId);
      qc.setQueryData<PrepState>(["prep", jobId], s);
      toast.success("Prep pack ready");
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const reviewed = stories.filter((s) => !s.draft);
  const latestFor = (question: string) => prep?.practice.find((p) => p.question === question);

  if (!prep) return null;
  if (!prep.pack) {
    return (
      <div className="flex flex-1 items-center justify-center p-8">
        <div className="max-w-md text-center">
          <h2 className="text-headline font-semibold">{busy ? "Preparing your pack…" : `Prepare for ${job?.company ?? "this interview"}`}</h2>
          <p className="mt-2 text-callout text-ink-2">
            Waypoint reads the posting and your profile, then lists the questions you're likely to get, the story to tell for each, honest answers for your gaps, and what to ask them.
          </p>
          {reviewed.length === 0 && <p className="mt-3 rounded-[9px] bg-amber/10 px-3 py-2 text-callout text-amber">Your story bank is empty, so questions won't be matched to stories. You can add stories first in the Story bank tab.</p>}
          <Button className="mt-5" variant="primary" busy={busy} icon={<Sparkles className="size-3.5" />} onClick={build}>Build prep pack</Button>
          {busy && <p className="mt-3 text-caption text-ink-3">This takes about a minute.</p>}
        </div>
      </div>
    );
  }

  const pack = prep.pack;
  return (
    <div className="scroll flex-1">
      <div className="mx-auto grid max-w-[1180px] grid-cols-[1fr_300px] gap-8 px-8 py-6">
        <div className="flex min-w-0 flex-col gap-8">
          <section>
            <div className="flex flex-wrap gap-1.5">{pack.role_focus.map((f) => <Tag key={f} tone="indigo">{f}</Tag>)}</div>
            <p className="selectable mt-3 text-body text-ink-2">{pack.company_brief}</p>
          </section>
          {ORDER.map((cat) => {
            const qs = pack.questions.filter((q) => q.category === cat);
            if (!qs.length) return null;
            return (
              <section key={cat}>
                <SectionTitle>{CATEGORY[cat]}</SectionTitle>
                <ul className="flex flex-col gap-2.5">
                  {qs.map((q) => (
                    <QuestionCard key={q.question} q={q} story={stories.find((s) => s.id === q.story_id)} practice={latestFor(q.question)} jobId={jobId} onPracticed={() => qc.invalidateQueries({ queryKey: ["prep", jobId] })} />
                  ))}
                </ul>
              </section>
            );
          })}
          {pack.gaps.length > 0 && (
            <section>
              <SectionTitle>Your gaps, answered honestly</SectionTitle>
              <ul className="flex flex-col gap-2.5">
                {pack.gaps.map((g) => (
                  <li key={g.gap} className="rounded-[12px] bg-amber/8 p-4">
                    <p className="text-callout font-semibold">{g.gap}</p>
                    <p className="selectable mt-1 text-callout text-ink">{g.answer}</p>
                  </li>
                ))}
              </ul>
            </section>
          )}
          <p className="pb-6 text-caption text-ink-3">Built {prep.updated_at ? new Date(prep.updated_at).toLocaleString() : ""} with {prep.model}. Talking points come from your profile; check every one before you say it.</p>
        </div>

        <aside className="flex flex-col gap-5">
          <Panel className="p-4">
            <SectionTitle>Before the interview</SectionTitle>
            <ul className="flex flex-col gap-1.5">
              {pack.checklist.map((c) => {
                const on = done.includes(c);
                return (
                  <li key={c}>
                    <button
                      className="flex w-full items-start gap-2 text-left text-callout"
                      onClick={() => {
                        const next = on ? done.filter((x) => x !== c) : [...done, c];
                        setDone(next);
                        persist(next, debrief);
                      }}
                      aria-pressed={on}
                    >
                      <span className={clsx("mt-0.5 flex size-4 shrink-0 items-center justify-center rounded-[5px] border", on ? "border-teal bg-teal text-white" : "border-fill-strong")}>{on && <Check className="size-3" />}</span>
                      <span className={clsx(on && "text-ink-3 line-through")}>{c}</span>
                    </button>
                  </li>
                );
              })}
            </ul>
          </Panel>
          <Panel className="p-4">
            <SectionTitle>Questions to ask them</SectionTitle>
            <ul className="flex flex-col gap-2">{pack.questions_to_ask.map((q) => <li key={q} className="selectable text-callout">{q}</li>)}</ul>
          </Panel>
          <Panel className="p-4">
            <SectionTitle>Debrief</SectionTitle>
            <AutoTextarea
              value={debrief}
              onChange={(e) => {
                setDebrief(e.target.value);
                persist(done, e.target.value);
              }}
              placeholder="Afterwards: who you met, what they asked, what you'd answer differently."
              className="min-h-24 text-callout"
            />
            <p className="mt-1.5 text-caption text-ink-3">Saved as you type, and used when you draft the thank-you note.</p>
          </Panel>
          <Button variant="plain" busy={busy} icon={<RefreshCw className="size-3.5" />} onClick={build}>Rebuild prep pack</Button>
        </aside>
      </div>
    </div>
  );
}

const blankStory = (): Story => ({ id: 0, title: "", source: "", situation: "", task: "", action: "", result: "", reflection: "", tags: [], draft: false, updated_at: null });

function StoryBank() {
  const qc = useQueryClient();
  const { data: stories = [] } = useQuery({ queryKey: ["stories"], queryFn: api.storiesList });
  const [selected, setSelected] = useState<number | "new" | null>(null);
  const [form, setForm] = useState<Story>(blankStory());
  const [tagText, setTagText] = useState("");
  const [busy, setBusy] = useState<"suggest" | "save" | null>(null);

  const current = useMemo(() => (selected === "new" ? null : stories.find((s) => s.id === selected) ?? null), [selected, stories]);
  useEffect(() => {
    if (selected == null && stories.length) setSelected(stories[0].id);
  }, [stories, selected]);
  useEffect(() => {
    const s = current ?? blankStory();
    setForm(s);
    setTagText(s.tags.join(", "));
  }, [current, selected]);

  const refresh = () => qc.invalidateQueries({ queryKey: ["stories"] });

  const suggest = async () => {
    setBusy("suggest");
    try {
      const r = await api.storiesSuggest();
      refresh();
      toast.success(`${r.added} ${r.added === 1 ? "story" : "stories"} drafted from your profile`, { description: r.flagged.length ? `Check the numbers in: ${r.flagged.join("; ")}` : "Fill in the bracketed parts, then save each one." });
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
    }
  };

  const save = async () => {
    setBusy("save");
    try {
      const id = await api.storySave({ ...form, tags: tagText.split(",").map((t) => t.trim()).filter(Boolean) });
      await refresh();
      setSelected(id);
      toast.success("Story saved");
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
    }
  };

  const remove = async () => {
    if (!current) return;
    await api.storyDelete(current.id);
    setSelected(null);
    refresh();
  };

  const field = (key: "situation" | "task" | "action" | "result", label: string, hint: string) => (
    <Field label={label} hint={hint}>
      <AutoTextarea value={form[key]} onChange={(e) => setForm({ ...form, [key]: e.target.value })} className="min-h-16 text-callout" />
    </Field>
  );

  return (
    <div className="flex min-h-0 flex-1">
      <div className="scroll w-[320px] shrink-0 border-r border-hairline p-3">
        <div className="mb-3 flex gap-2">
          <Button size="sm" className="flex-1" busy={busy === "suggest"} icon={<Sparkles className="size-3" />} onClick={suggest}>Suggest from profile</Button>
          <Button size="sm" variant="plain" icon={<Plus className="size-3" />} onClick={() => setSelected("new")}>New</Button>
        </div>
        {stories.length === 0 && <p className="px-2 py-6 text-center text-callout text-ink-3">Stories are the examples you tell in interviews. Start with a few from your profile.</p>}
        <ul className="flex flex-col gap-1">
          {stories.map((s) => (
            <li key={s.id}>
              <button
                onClick={() => setSelected(s.id)}
                className={clsx("w-full rounded-[9px] px-3 py-2 text-left transition-colors", selected === s.id ? "bg-selection text-white" : "hover:bg-fill")}
              >
                <span className="flex items-center gap-2">
                  <span className="min-w-0 flex-1 truncate text-callout font-semibold">{s.title}</span>
                  {s.draft && <span className={clsx("text-caption font-medium", selected === s.id ? "text-white/80" : "text-amber")}>Draft</span>}
                </span>
                <span className={clsx("block truncate text-caption", selected === s.id ? "text-white/75" : "text-ink-3")}>{s.tags.join(", ") || s.source}</span>
              </button>
            </li>
          ))}
        </ul>
      </div>
      <div className="scroll min-w-0 flex-1">
        {selected == null ? null : (
          <div className="mx-auto flex max-w-[720px] flex-col gap-4 px-8 py-6">
            {form.draft && <p className="rounded-[9px] bg-amber/10 px-3 py-2 text-callout text-amber">Drafted from your profile. Replace anything in [brackets] with what really happened, then save.</p>}
            <div className="grid grid-cols-[1fr_260px] gap-3">
              <Field label="Title"><Input value={form.title} onChange={(e) => setForm({ ...form, title: e.target.value })} placeholder="Zero-downtime Postgres migration" /></Field>
              <Field label="From"><Input value={form.source} onChange={(e) => setForm({ ...form, source: e.target.value })} placeholder="Senior Engineer, Acme" /></Field>
            </div>
            {field("situation", "Situation", "The context and what was at stake.")}
            {field("task", "Task", "What you were responsible for.")}
            {field("action", "Action", "What you did, in the first person.")}
            {field("result", "Result", "The outcome, with numbers if you have them.")}
            <Field label="Notes" hint="What you learned, or what you'd do differently. Interviewers often ask.">
              <AutoTextarea value={form.reflection} onChange={(e) => setForm({ ...form, reflection: e.target.value })} className="min-h-12 text-callout" />
            </Field>
            <Field label="Good for" hint="Separate with commas, e.g. ownership, conflict, scale">
              <Input value={tagText} onChange={(e) => setTagText(e.target.value)} />
            </Field>
            <div className="flex items-center gap-2 pb-6">
              {current && <Button variant="danger" icon={<Trash2 className="size-3.5" />} onClick={remove}>Delete</Button>}
              <div className="flex-1" />
              <Button variant="primary" busy={busy === "save"} onClick={save}>Save story</Button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export function Interviews({ initialJob }: { initialJob: number | null }) {
  const [tab, setTab] = useState<"prep" | "stories">("prep");
  const [jobId, setJobId] = useState<number | null>(initialJob);
  useEffect(() => {
    if (initialJob != null) {
      setJobId(initialJob);
      setTab("prep");
    }
  }, [initialJob]);
  const { data: jobs = [] } = useQuery({
    queryKey: ["jobs", "interview-picker"],
    queryFn: () => api.listJobs({ statuses: ["responded", "assessment", "interview", "offer", "applied"], limit: 300 }),
  });
  useEffect(() => {
    if (jobId == null && jobs.length) setJobId((jobs.find((j) => j.status === "interview") ?? jobs[0]).id);
  }, [jobs, jobId]);
  const job = jobs.find((j) => j.id === jobId);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="drag flex h-[52px] shrink-0 items-center gap-3 border-b border-hairline px-4" data-tauri-drag-region>
        <h1 className="text-headline font-semibold" data-tauri-drag-region>Interviews</h1>
        <Segmented value={tab} onChange={setTab} options={[{ value: "prep", label: "Prep" }, { value: "stories", label: "Story bank" }]} />
        {tab === "prep" && <JobPicker jobs={jobs} jobId={jobId} onChange={setJobId} />}
      </header>
      {tab === "stories" ? (
        <StoryBank />
      ) : jobId == null ? (
        <div className="flex flex-1 items-center justify-center p-8 text-center">
          <div className="max-w-sm">
            <p className="text-body font-medium">No interviews yet</p>
            <p className="mt-1 text-callout text-ink-3">When a role moves to Responded or Interview, prepare for it here. Meanwhile, build your story bank.</p>
          </div>
        </div>
      ) : (
        <PrepView key={jobId} jobId={jobId} job={job} />
      )}
    </div>
  );
}
