import { useQuery, useQueryClient } from "@tanstack/react-query";
import clsx from "clsx";
import { BellOff, Check, ChevronRight, Clock, Copy, ExternalLink, Sparkles, X } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { Button, Input, Segmented, Textarea } from "../components/ui";
import { api, errorMessage } from "../lib/api";
import { STATUS_LABEL } from "../lib/format";
import type { FollowUpAction, FollowUpItem, Touch } from "../lib/types";

const ACTION: Record<FollowUpAction, string> = {
  follow_up: "Follow up",
  reply: "Reply to them",
  thank_you: "Send a thank-you note",
};

const LOGGED: Record<FollowUpAction, string> = {
  follow_up: "Follow-up",
  reply: "Reply",
  thank_you: "Thank-you note",
};

const CHANNELS = [
  { value: "email", label: "Email" },
  { value: "linkedin", label: "LinkedIn" },
  { value: "phone", label: "Phone" },
  { value: "other", label: "Other" },
];

const day = (iso: string) => new Date(`${iso}T12:00:00`);
const daysBetween = (a: string, b: string) => Math.round((day(b).getTime() - day(a).getTime()) / 86_400_000);

/** "Due today", "2 days overdue", "Due Thu 24 Oct". */
function when(item: FollowUpItem, today: string): string {
  if (item.urgency === "off") return "Reminders off";
  if (item.urgency === "cold") return `No answer after ${item.touches} follow-ups`;
  if (item.urgency === "urgent") return "Reply today";
  if (!item.next_on) return "";
  const d = daysBetween(today, item.next_on);
  if (d < 0) return `${-d} ${d === -1 ? "day" : "days"} overdue`;
  if (d === 0) return "Due today";
  if (d === 1) return "Due tomorrow";
  return `Due ${day(item.next_on).toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short" })}`;
}

function since(item: FollowUpItem): string {
  const status = STATUS_LABEL[item.status];
  const days = item.days_in_status === 0 ? "today" : item.days_in_status === 1 ? "yesterday" : `${item.days_in_status} days ago`;
  const touches = item.touches ? `, ${item.touches} ${item.touches === 1 ? "touch" : "touches"} logged` : "";
  return `${status} ${days}${touches}`;
}

function Detail({ item, onChanged, onOpenJob }: { item: FollowUpItem; onChanged: () => void; onOpenJob: (id: number) => void }) {
  const { data: history = [], refetch } = useQuery({ queryKey: ["touches", item.job_id], queryFn: () => api.followupHistory(item.job_id) });
  const [notes, setNotes] = useState("");
  const [draft, setDraft] = useState<{ subject: string; body: string } | null>(null);
  const [channel, setChannel] = useState("email");
  const [busy, setBusy] = useState<"draft" | "log" | null>(null);

  const run = async (fn: () => Promise<unknown>, done?: string) => {
    try {
      await fn();
      if (done) toast.success(done);
      onChanged();
      refetch();
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };

  const write = async () => {
    setBusy("draft");
    try {
      setDraft(await api.followupDraft(item.job_id, item.action, notes));
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
    }
  };

  const copy = async () => {
    if (!draft) return;
    const text = draft.subject ? `Subject: ${draft.subject}\n\n${draft.body}` : draft.body;
    try {
      await navigator.clipboard.writeText(text);
      toast.success("Copied. Paste it into your email or LinkedIn.");
    } catch {
      toast.error("Couldn't copy. Select the text and copy it instead.");
    }
  };

  const log = async () => {
    setBusy("log");
    const note = draft?.subject || notes.trim();
    await run(() => api.followupLog(item.job_id, item.action, channel, note), `${LOGGED[item.action]} logged for ${item.company}`);
    setDraft(null);
    setNotes("");
    setBusy(null);
  };

  return (
    <div className="grid grid-cols-[1fr_260px] gap-6 border-t border-hairline bg-window/60 px-5 py-4">
      <div className="flex min-w-0 flex-col gap-3">
        {item.urgency !== "off" && (
          <>
            <Input value={notes} onChange={(e) => setNotes(e.target.value)} placeholder={item.action === "thank_you" ? "Who did you meet, and what stood out? (optional)" : "Anything to mention? (optional)"} className="text-callout" />
            {draft ? (
              <div className="flex flex-col gap-2">
                {draft.subject !== "" && <Input value={draft.subject} onChange={(e) => setDraft({ ...draft, subject: e.target.value })} className="font-medium" aria-label="Subject" />}
                <Textarea value={draft.body} onChange={(e) => setDraft({ ...draft, body: e.target.value })} className="min-h-40 text-callout" aria-label="Message" />
                <p className="text-caption text-ink-3">Check every detail before you send it. Waypoint never sends anything for you.</p>
              </div>
            ) : null}
            <div className="flex items-center gap-2">
              <Button size="sm" busy={busy === "draft"} icon={<Sparkles className="size-3" />} onClick={write}>
                {draft ? "Draft again" : "Draft the message"}
              </Button>
              {draft && <Button size="sm" variant="plain" icon={<Copy className="size-3" />} onClick={copy}>Copy</Button>}
            </div>
            <div className="flex items-center gap-2 border-t border-hairline pt-3">
              <span className="text-callout text-ink-2">Sent it? By</span>
              <Segmented value={channel} onChange={setChannel} options={CHANNELS} />
              <div className="flex-1" />
              <Button size="sm" variant="primary" busy={busy === "log"} icon={<Check className="size-3" />} onClick={log}>
                Mark as sent
              </Button>
            </div>
          </>
        )}
      </div>
      <div className="flex flex-col gap-3 text-callout">
        <div>
          <p className="mb-1.5 text-caption font-semibold text-ink-2">Remind me later</p>
          <div className="flex flex-wrap gap-1.5">
            {[3, 7, 14].map((d) => (
              <Button key={d} size="sm" icon={<Clock className="size-3" />} onClick={() => run(() => api.followupSnooze(item.job_id, d), `Snoozed for ${d === 7 ? "a week" : d === 14 ? "two weeks" : `${d} days`}`)}>
                {d === 7 ? "1 week" : d === 14 ? "2 weeks" : `${d} days`}
              </Button>
            ))}
            {item.snoozed && <Button size="sm" variant="plain" onClick={() => run(() => api.followupSnooze(item.job_id, null), "Snooze cleared")}>Clear snooze</Button>}
          </div>
        </div>
        <button
          className="flex items-center gap-1.5 self-start text-callout text-ink-2 hover:text-ink"
          onClick={() => run(() => api.followupSetOff(item.job_id, item.urgency !== "off"), item.urgency === "off" ? "Reminders back on" : "Reminders off for this job")}
        >
          <BellOff className="size-3.5" />
          {item.urgency === "off" ? "Turn reminders back on" : "Stop reminders for this job"}
        </button>
        <button className="flex items-center gap-1.5 self-start text-callout text-ink-2 hover:text-ink" onClick={() => onOpenJob(item.job_id)}>
          <ExternalLink className="size-3.5" />
          Open the job
        </button>
        {history.length > 0 && <History touches={history} onDelete={(id) => run(() => api.followupDelete(id))} />}
      </div>
    </div>
  );
}

function History({ touches, onDelete }: { touches: Touch[]; onDelete: (id: number) => void }) {
  return (
    <div>
      <p className="mb-1 text-caption font-semibold text-ink-2">Logged</p>
      <ul className="flex flex-col gap-1">
        {touches.map((t) => (
          <li key={t.id} className="group flex items-start gap-1.5 text-caption text-ink-2">
            <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-teal" />
            <span className="min-w-0 flex-1">
              {LOGGED[t.kind] ?? t.kind} by {CHANNELS.find((c) => c.value === t.channel)?.label.toLowerCase() ?? t.channel}, {new Date(t.at).toLocaleDateString(undefined, { day: "numeric", month: "short" })}
              {t.note && <span className="block truncate text-ink-3">{t.note}</span>}
            </span>
            <button aria-label="Remove this entry" className="opacity-0 transition-opacity group-hover:opacity-100" onClick={() => onDelete(t.id)}>
              <X className="size-3 text-ink-3 hover:text-coral" />
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}

function Row({ item, today, open, onToggle, onChanged, onOpenJob }: { item: FollowUpItem; today: string; open: boolean; onToggle: () => void; onChanged: () => void; onOpenJob: (id: number) => void }) {
  const hot = item.urgency === "urgent" || item.urgency === "due";
  return (
    <li className="overflow-hidden rounded-[12px] bg-surface shadow-[0_0_0_0.5px_var(--color-hairline),0_1px_2px_rgb(0_0_0/0.04)]">
      <button onClick={onToggle} aria-expanded={open} className="flex w-full items-center gap-4 px-4 py-3 text-left transition-colors hover:bg-raised">
        <span className={clsx("size-2 shrink-0 rounded-full", item.urgency === "urgent" ? "bg-coral" : hot ? "bg-amber" : item.urgency === "waiting" ? "bg-teal" : "bg-fill-strong")} />
        <span className="min-w-0 flex-1">
          <span className="block truncate text-caption font-medium text-ink-2">{item.company}</span>
          <span className="block truncate text-body font-semibold">{item.title}</span>
          <span className="block truncate text-caption text-ink-3">{since(item)}</span>
        </span>
        <span className="shrink-0 text-right">
          {item.urgency !== "cold" && item.urgency !== "off" && <span className="block text-callout font-medium">{ACTION[item.action]}</span>}
          <span className={clsx("block text-caption", item.urgency === "urgent" ? "text-coral" : hot ? "text-amber" : "text-ink-3")}>
            {item.snoozed ? `Snoozed until ${day(item.next_on!).toLocaleDateString(undefined, { day: "numeric", month: "short" })}` : when(item, today)}
          </span>
        </span>
        <ChevronRight className={clsx("size-4 shrink-0 text-ink-3 transition-transform", open && "rotate-90")} />
      </button>
      {open && <Detail item={item} onChanged={onChanged} onOpenJob={onOpenJob} />}
    </li>
  );
}

const GROUPS: { title: string; hint: string; test: (i: FollowUpItem) => boolean }[] = [
  { title: "Needs you now", hint: "Replies first, then follow-ups that are due.", test: (i) => i.urgency === "urgent" || i.urgency === "due" },
  { title: "Coming up", hint: "Nothing to do yet. Waypoint will remind you.", test: (i) => i.urgency === "waiting" },
  { title: "Gone quiet", hint: "Two follow-ups with no answer. Snooze one to try again, or move it to Rejected.", test: (i) => i.urgency === "cold" },
  { title: "Reminders off", hint: "", test: (i) => i.urgency === "off" },
];

export function FollowUps({ onOpenJob }: { onOpenJob: (id: number) => void }) {
  const qc = useQueryClient();
  const { data } = useQuery({ queryKey: ["followups"], queryFn: api.followups, refetchInterval: 60_000 });
  const [open, setOpen] = useState<number | null>(null);
  const changed = () => {
    qc.invalidateQueries({ queryKey: ["followups"] });
  };

  if (!data) return null;
  if (data.items.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center p-8">
        <div className="max-w-sm text-center">
          <p className="text-body font-medium">No follow-ups yet</p>
          <p className="mt-1 text-callout text-ink-3">Move a job to Applied and Waypoint tells you when to check in: a week after applying, a day after they reply, and the day after an interview.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="scroll flex-1">
      <div className="mx-auto flex max-w-[860px] flex-col gap-7 px-8 py-6">
        {GROUPS.map((g) => {
          const items = data.items.filter(g.test);
          if (!items.length) return null;
          return (
            <section key={g.title}>
              <div className="mb-2.5 flex items-baseline gap-2">
                <h2 className="text-headline font-semibold">{g.title}</h2>
                <span className="tabular text-callout text-ink-3">{items.length}</span>
              </div>
              {g.hint && <p className="-mt-1.5 mb-3 text-callout text-ink-3">{g.hint}</p>}
              <ul className="flex flex-col gap-2">
                {items.map((i) => (
                  <Row key={i.job_id} item={i} today={data.today} open={open === i.job_id} onToggle={() => setOpen(open === i.job_id ? null : i.job_id)} onChanged={changed} onOpenJob={onOpenJob} />
                ))}
              </ul>
            </section>
          );
        })}
      </div>
    </div>
  );
}
