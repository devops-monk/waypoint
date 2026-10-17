import { useQuery, useQueryClient } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { FileUp, Plus, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { Button, Field, IconButton, Input, Panel, Textarea, Toggle } from "../components/ui";
import { api, errorMessage } from "../lib/api";
import type { Experience, Profile } from "../lib/types";

const lines = (v: string[]) => v.join("\n");
const fromLines = (s: string) => s.split("\n").map((x) => x.trim()).filter(Boolean);
const csv = (v: string[]) => v.join(", ");
const fromCsv = (s: string) => s.split(",").map((x) => x.trim()).filter(Boolean);

function ListField({ label, hint, value, onChange, comma, rows = 3 }: { label: string; hint?: string; value: string[]; onChange: (v: string[]) => void; comma?: boolean; rows?: number }) {
  // Edit as text so typing separators doesn't fight the parser; parse on blur.
  const [text, setText] = useState(comma ? csv(value) : lines(value));
  useEffect(() => setText(comma ? csv(value) : lines(value)), [value, comma]);
  return (
    <Field label={label} hint={hint}>
      {comma ? (
        <Input value={text} onChange={(e) => setText(e.target.value)} onBlur={() => onChange(fromCsv(text))} />
      ) : (
        <Textarea rows={rows} value={text} onChange={(e) => setText(e.target.value)} onBlur={() => onChange(fromLines(text))} className="min-h-0 text-callout" />
      )}
    </Field>
  );
}

function ExperienceEditor({ value, onChange }: { value: Experience[]; onChange: (v: Experience[]) => void }) {
  const update = (i: number, patch: Partial<Experience>) => onChange(value.map((e, j) => (j === i ? { ...e, ...patch } : e)));
  return (
    <div className="flex flex-col gap-3">
      {value.map((e, i) => (
        <Panel key={i} className="p-4">
          <div className="grid grid-cols-[1fr_1fr_110px_110px_auto] items-end gap-3">
            <Field label="Title"><Input value={e.title} onChange={(x) => update(i, { title: x.target.value })} /></Field>
            <Field label="Company"><Input value={e.company} onChange={(x) => update(i, { company: x.target.value })} /></Field>
            <Field label="Start"><Input value={e.start} onChange={(x) => update(i, { start: x.target.value })} placeholder="2021-03" /></Field>
            <Field label="End"><Input value={e.end} onChange={(x) => update(i, { end: x.target.value })} placeholder="Present" /></Field>
            <IconButton label="Remove role" className="mb-0.5" onClick={() => onChange(value.filter((_, j) => j !== i))}><Trash2 className="size-3.5" /></IconButton>
          </div>
          <div className="mt-3">
            <ListField label="What you did" hint="One achievement per line. Numbers you write here are the only numbers Waypoint will ever use." value={e.highlights} onChange={(h) => update(i, { highlights: h })} rows={4} />
          </div>
        </Panel>
      ))}
      <Button size="sm" className="self-start" icon={<Plus className="size-3.5" />} onClick={() => onChange([...value, { company: "", title: "", location: "", start: "", end: "", highlights: [] }])}>
        Add role
      </Button>
    </div>
  );
}

export function ProfileView() {
  const qc = useQueryClient();
  const { data: saved } = useQuery({ queryKey: ["profile"], queryFn: api.getProfile });
  const [p, setP] = useState<Profile | null>(null);
  const [importing, setImporting] = useState(false);
  const [paste, setPaste] = useState("");
  const [showPaste, setShowPaste] = useState(false);
  const [draftFromResume, setDraftFromResume] = useState(false);

  useEffect(() => {
    if (saved && !p) setP(saved);
  }, [saved, p]);

  if (!p) return null;
  const dirty = JSON.stringify(p) !== JSON.stringify(saved);
  const set = <K extends keyof Profile>(k: K, v: Profile[K]) => setP({ ...p, [k]: v });
  const pref = <K extends keyof Profile["preferences"]>(k: K, v: Profile["preferences"][K]) => setP({ ...p, preferences: { ...p.preferences, [k]: v } });

  const runImport = async (args: { path?: string; text?: string }) => {
    setImporting(true);
    try {
      const draft = await api.importResume(args);
      setP(draft);
      setDraftFromResume(true);
      setShowPaste(false);
      toast.success("Resume read", { description: "Check every field below, then save." });
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setImporting(false);
    }
  };

  const pickFile = async () => {
    const path = await open({ multiple: false, filters: [{ name: "Resume", extensions: ["pdf", "txt", "md"] }] });
    if (typeof path === "string") runImport({ path });
  };

  const save = async () => {
    try {
      await api.saveProfile(p);
      await qc.invalidateQueries({ queryKey: ["profile"] });
      setDraftFromResume(false);
      toast.success("Profile saved");
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="drag flex h-[52px] shrink-0 items-center gap-3 border-b border-hairline px-4" data-tauri-drag-region>
        <h1 className="text-headline font-semibold" data-tauri-drag-region>Profile</h1>
        <div className="flex-1" data-tauri-drag-region />
        {dirty && (
          <Button variant="plain" onClick={() => (setP(saved ?? null), setDraftFromResume(false))}>
            Discard changes
          </Button>
        )}
        <Button variant="primary" disabled={!dirty} onClick={save}>
          Save profile
        </Button>
      </header>
      <div className="scroll flex-1">
        <div className="mx-auto flex max-w-[820px] flex-col gap-8 px-8 py-7">
          <Panel className="flex items-center gap-5 p-5">
            <div className="flex size-11 shrink-0 items-center justify-center rounded-full bg-gradient-to-br from-indigo to-teal text-white">
              <FileUp className="size-5" />
            </div>
            <div className="flex-1">
              <p className="text-body font-semibold">{p.experience.length ? "Update from a newer resume" : "Start from your resume"}</p>
              <p className="mt-0.5 text-callout text-ink-2">Waypoint copies facts as written and adds nothing. You review everything before it's saved.</p>
            </div>
            <Button variant="plain" onClick={() => setShowPaste((s) => !s)}>Paste text</Button>
            <Button variant="primary" busy={importing} onClick={pickFile}>Choose file</Button>
          </Panel>
          {showPaste && (
            <div className="-mt-5 flex flex-col gap-2">
              <Textarea value={paste} onChange={(e) => setPaste(e.target.value)} placeholder="Paste your resume text" className="min-h-48 text-callout" />
              <Button className="self-end" busy={importing} disabled={paste.trim().length < 200} onClick={() => runImport({ text: paste })}>Read resume</Button>
            </div>
          )}
          {draftFromResume && <p className="-mt-4 rounded-[9px] bg-indigo/10 px-3 py-2 text-callout text-indigo">This is a draft from your resume. Correct anything that's wrong, then save.</p>}

          <section className="grid grid-cols-2 gap-4">
            <h2 className="col-span-2 text-headline font-semibold">About you</h2>
            <Field label="Name"><Input value={p.name} onChange={(e) => set("name", e.target.value)} /></Field>
            <Field label="Email"><Input value={p.email} onChange={(e) => set("email", e.target.value)} /></Field>
            <Field label="Headline"><Input value={p.headline} onChange={(e) => set("headline", e.target.value)} placeholder="Senior backend engineer" /></Field>
            <Field label="Location"><Input value={p.location} onChange={(e) => set("location", e.target.value)} placeholder="Bengaluru, India" /></Field>
            <Field label="Summary" className="col-span-2"><Textarea value={p.summary} onChange={(e) => set("summary", e.target.value)} className="text-callout" /></Field>
            <div className="col-span-2"><ListField label="Skills" hint="Separate with commas" comma value={p.skills} onChange={(v) => set("skills", v)} /></div>
            <div className="col-span-2">
              <Field label="Languages" hint="One per line, with your level, for example: English, C1">
                <Textarea
                  rows={2}
                  className="min-h-0 text-callout"
                  defaultValue={p.languages.map((l) => (l.level ? `${l.name}, ${l.level}` : l.name)).join("\n")}
                  key={p.languages.map((l) => l.name).join()}
                  onBlur={(e) => set("languages", fromLines(e.target.value).map((line) => { const [name, ...rest] = line.split(","); return { name: name.trim(), level: rest.join(",").trim() }; }))}
                />
              </Field>
            </div>
          </section>

          <section>
            <h2 className="mb-3 text-headline font-semibold">Experience</h2>
            <ExperienceEditor value={p.experience} onChange={(v) => set("experience", v)} />
          </section>

          <section className="grid grid-cols-2 gap-4">
            <div className="col-span-2">
              <h2 className="text-headline font-semibold">What you're looking for</h2>
              <p className="mt-0.5 text-callout text-ink-2">Used for the eligibility, language and location checks, and for career fit.</p>
            </div>
            <ListField label="Target roles" comma value={p.preferences.target_roles} onChange={(v) => pref("target_roles", v)} />
            <ListField label="Locations" comma value={p.preferences.locations} onChange={(v) => pref("locations", v)} />
            <Field label="Work authorization" hint="Citizenships and permits, for example: Indian citizen, UK Skilled Worker visa">
              <Input value={p.preferences.work_authorization} onChange={(e) => pref("work_authorization", e.target.value)} />
            </Field>
            <div className="grid grid-cols-[1fr_90px] gap-3">
              <Field label="Minimum salary">
                <Input type="number" value={p.preferences.salary_floor ?? ""} onChange={(e) => pref("salary_floor", e.target.value ? Number(e.target.value) : null)} />
              </Field>
              <Field label="Currency"><Input value={p.preferences.salary_currency} onChange={(e) => pref("salary_currency", e.target.value.toUpperCase())} placeholder="USD" /></Field>
            </div>
            <div className="col-span-2 flex flex-wrap gap-6">
              <label className="flex items-center gap-2.5 text-callout"><Toggle checked={p.preferences.remote_ok} onChange={(v) => pref("remote_ok", v)} label="Open to remote" />Open to remote</label>
              <label className="flex items-center gap-2.5 text-callout"><Toggle checked={p.preferences.relocation_ok} onChange={(v) => pref("relocation_ok", v)} label="Open to relocating" />Open to relocating</label>
              <label className="flex items-center gap-2.5 text-callout"><Toggle checked={p.preferences.needs_sponsorship} onChange={(v) => pref("needs_sponsorship", v)} label="Need visa sponsorship" />Need visa sponsorship</label>
            </div>
            <ListField label="Career goals" hint="One per line" value={p.preferences.career_goals} onChange={(v) => pref("career_goals", v)} />
            <ListField label="Deal-breakers" hint="One per line" value={p.preferences.deal_breakers} onChange={(v) => pref("deal_breakers", v)} />
            <ListField label="Work that energizes you" value={p.preferences.energizing} onChange={(v) => pref("energizing", v)} />
            <ListField label="Work that drains you" value={p.preferences.draining} onChange={(v) => pref("draining", v)} />
          </section>
        </div>
      </div>
    </div>
  );
}
