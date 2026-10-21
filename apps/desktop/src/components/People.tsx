import { useQuery, useQueryClient } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import clsx from "clsx";
import { Copy, ExternalLink, Search, Sparkles, Trash2, Upload, Users } from "lucide-react";
import { useMemo, useState } from "react";
import { toast } from "sonner";
import { api, errorMessage } from "../lib/api";
import type { Contact, ContactMatch } from "../lib/types";
import { AutoTextarea, Button, IconButton, Input, Panel, Segmented, SectionTitle } from "./ui";

const since = (c: Contact) => (c.connected_on ? `Connected ${new Date(`${c.connected_on}T12:00:00`).getFullYear()}` : "");
const name = (c: Contact) => `${c.first_name} ${c.last_name}`.trim();

function MessageDraft({ jobId, contact, onClose }: { jobId: number; contact: Contact; onClose: () => void }) {
  const [ask, setAsk] = useState<"referral" | "advice">("referral");
  const [notes, setNotes] = useState("");
  const [draft, setDraft] = useState<{ subject: string; body: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const write = async () => {
    setBusy(true);
    try {
      setDraft(await api.contactMessage(jobId, contact.id, ask, notes));
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };
  const copy = async () => {
    if (!draft) return;
    try {
      await navigator.clipboard.writeText(draft.body);
      toast.success("Copied. Paste it into LinkedIn or your email.");
    } catch {
      toast.error("Couldn't copy. Select the text and copy it instead.");
    }
  };
  return (
    <div className="mt-2 rounded-[10px] bg-fill p-3">
      <div className="flex flex-wrap items-center gap-2">
        <Segmented value={ask} onChange={setAsk} options={[{ value: "referral", label: "Ask for a referral" }, { value: "advice", label: "Ask for advice" }]} />
        <div className="flex-1" />
        <Button size="sm" variant="plain" onClick={onClose}>Close</Button>
      </div>
      <Input value={notes} onChange={(e) => setNotes(e.target.value)} placeholder={`How do you know ${contact.first_name}? (optional, e.g. "we worked together at Acme")`} className="mt-2 text-callout" />
      {draft && <AutoTextarea value={draft.body} onChange={(e) => setDraft({ ...draft, body: e.target.value })} className="mt-2 text-callout" aria-label="Message" />}
      <div className="mt-2 flex items-center gap-2">
        <Button size="sm" busy={busy} icon={<Sparkles className="size-3" />} onClick={write}>{draft ? "Draft again" : "Draft the message"}</Button>
        {draft && <Button size="sm" variant="plain" icon={<Copy className="size-3" />} onClick={copy}>Copy</Button>}
        {contact.url && <Button size="sm" variant="plain" icon={<ExternalLink className="size-3" />} onClick={() => openUrl(contact.url)}>Open profile</Button>}
      </div>
      {draft && <p className="mt-1.5 text-caption text-ink-3">Check it reads like you before you send it. Waypoint never sends anything.</p>}
    </div>
  );
}

/** "People you know here", shown on a job when the connections import has matches. */
export function PeopleForJob({ jobId, company }: { jobId: number; company: string }) {
  const { data: matches = [] } = useQuery({ queryKey: ["contacts-for-job", jobId], queryFn: () => api.contactsForJob(jobId) });
  const [open, setOpen] = useState<number | null>(null);
  const likely = matches.filter((m) => m.strength !== "weak");
  const maybe = matches.filter((m) => m.strength === "weak");
  if (!matches.length) return null;
  const row = (m: ContactMatch) => (
    <li key={m.contact.id} className="py-2">
      <div className="flex items-center gap-3">
        <span className="flex size-8 shrink-0 items-center justify-center rounded-full bg-indigo/12 text-caption font-semibold text-indigo" aria-hidden>
          {(m.contact.first_name[0] ?? "") + (m.contact.last_name[0] ?? "")}
        </span>
        <div className="min-w-0 flex-1">
          <p className="truncate text-callout font-medium">{name(m.contact)}</p>
          <p className="truncate text-caption text-ink-3">{[m.contact.position, m.strength === "weak" ? m.contact.company : "", since(m.contact)].filter(Boolean).join(", ")}</p>
        </div>
        <Button size="sm" onClick={() => setOpen(open === m.contact.id ? null : m.contact.id)}>Reach out</Button>
      </div>
      {open === m.contact.id && <MessageDraft jobId={jobId} contact={m.contact} onClose={() => setOpen(null)} />}
    </li>
  );
  return (
    <section className="mt-8">
      <SectionTitle>{likely.length ? `You know ${likely.length} ${likely.length === 1 ? "person" : "people"} at ${company}` : `People who may work at ${company}`}</SectionTitle>
      <Panel className="px-4 py-1">
        <ul className="divide-y divide-hairline">{likely.map(row)}</ul>
        {maybe.length > 0 && (
          <>
            {likely.length > 0 && <p className="border-t border-hairline pt-2 text-caption text-ink-3">Similar company names, possibly not the same employer</p>}
            <ul className="divide-y divide-hairline">{maybe.map(row)}</ul>
          </>
        )}
      </Panel>
      <p className="mt-1.5 text-caption text-ink-3">A referral is often the fastest way past the first screen. Ask the person you know best.</p>
    </section>
  );
}

/** The People tab: import, browse and remove your connections. */
export function PeopleView() {
  const qc = useQueryClient();
  const { data: contacts = [] } = useQuery({ queryKey: ["contacts"], queryFn: api.contactsList });
  const [q, setQ] = useState("");
  const [busy, setBusy] = useState(false);
  const refresh = () => {
    qc.invalidateQueries({ queryKey: ["contacts"] });
    qc.invalidateQueries({ queryKey: ["contacts-for-job"] });
  };

  const importFile = async () => {
    const path = await open({ multiple: false, filters: [{ name: "LinkedIn connections", extensions: ["csv"] }] });
    if (!path || Array.isArray(path)) return;
    setBusy(true);
    try {
      const r = await api.contactsImport(path);
      refresh();
      toast.success(`${r.added} people added${r.updated ? `, ${r.updated} updated` : ""}`, { description: r.without_company ? `${r.without_company} had no company listed and were skipped.` : undefined });
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const clear = async () => {
    const n = await api.contactsClear();
    refresh();
    toast(`Removed ${n} people from Waypoint`);
  };

  const shown = useMemo(() => {
    const t = q.trim().toLowerCase();
    return (t ? contacts.filter((c) => `${name(c)} ${c.company} ${c.position}`.toLowerCase().includes(t)) : contacts).slice(0, 300);
  }, [contacts, q]);
  const companies = useMemo(() => new Set(contacts.map((c) => c.company.toLowerCase())).size, [contacts]);

  return (
    <div className="flex flex-col gap-6">
      <Panel className="flex items-start gap-4 p-5">
        <span className="flex size-10 shrink-0 items-center justify-center rounded-[10px] bg-indigo/12 text-indigo"><Users className="size-5" /></span>
        <div className="flex-1">
          <p className="text-body font-semibold">{contacts.length ? `${contacts.length.toLocaleString()} people at ${companies.toLocaleString()} companies` : "See who you know at each company"}</p>
          <p className="mt-1 text-callout text-ink-2">
            In LinkedIn, open Settings, then Data privacy, then Get a copy of your data, and choose Connections. LinkedIn emails you a Connections.csv file; import it here. Waypoint never logs in to LinkedIn.
          </p>
          <p className="mt-1.5 text-caption text-ink-3">These are other people's details. They stay on this computer, and you can remove them all at any time.</p>
        </div>
        <div className="flex shrink-0 flex-col items-end gap-2">
          <Button variant="primary" busy={busy} icon={<Upload className="size-3.5" />} onClick={importFile}>{contacts.length ? "Import again" : "Import Connections.csv"}</Button>
          {contacts.length > 0 && <Button size="sm" variant="danger" icon={<Trash2 className="size-3" />} onClick={clear}>Remove all</Button>}
        </div>
      </Panel>
      {contacts.length > 0 && (
        <section>
          <div className="mb-3 flex items-center justify-between gap-4">
            <h2 className="text-headline font-semibold">People</h2>
            <div className="relative w-64">
              <Search className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-ink-3" />
              <Input value={q} onChange={(e) => setQ(e.target.value)} placeholder="Find a person or company" className="h-7 pl-7 text-callout" />
            </div>
          </div>
          <Panel className="divide-y divide-hairline">
            {shown.map((c) => (
              <div key={c.id} className="flex items-center gap-3 px-4 py-2">
                <div className="min-w-0 flex-1">
                  <p className="truncate text-callout font-medium">{name(c)}</p>
                  <p className="truncate text-caption text-ink-3">{[c.position, c.company].filter(Boolean).join(" at ")}</p>
                </div>
                <span className={clsx("text-caption text-ink-3")}>{since(c)}</span>
                {c.url && <IconButton label={`Open ${name(c)}'s profile`} onClick={() => openUrl(c.url)}><ExternalLink className="size-3.5" /></IconButton>}
              </div>
            ))}
          </Panel>
          {contacts.length > shown.length && !q && <p className="mt-2 text-caption text-ink-3">Showing the first 300. Search to find anyone else.</p>}
        </section>
      )}
    </div>
  );
}
