import { useQueryClient } from "@tanstack/react-query";
import { AnimatePresence, motion } from "motion/react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { api, errorMessage } from "../lib/api";
import { Button, Field, Input, Textarea } from "./ui";

const EMPTY = { url: "", company: "", title: "", location: "", description: "" };

/** Save a job found anywhere: LinkedIn, a referral, an email. Paste the posting text so it can be scored. */
export function AddJobSheet({ open, onClose, onAdded }: { open: boolean; onClose: () => void; onAdded: (id: number) => void }) {
  const [form, setForm] = useState(EMPTY);
  const [busy, setBusy] = useState(false);
  const qc = useQueryClient();

  useEffect(() => {
    if (open) setForm(EMPTY);
  }, [open]);

  const set = (k: keyof typeof EMPTY) => (e: { target: { value: string } }) => setForm((f) => ({ ...f, [k]: e.target.value }));

  const submit = async () => {
    setBusy(true);
    try {
      const id = await api.addJobManual(form);
      await qc.invalidateQueries();
      toast.success("Job saved");
      onClose();
      onAdded(id);
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <AnimatePresence>
      {open && (
        <motion.div className="fixed inset-0 z-40 flex justify-center bg-black/15 dark:bg-black/35" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} onMouseDown={onClose}>
          {/* A sheet that drops from the title bar, as on macOS. */}
          <motion.div
            role="dialog"
            aria-label="Add a job"
            initial={{ y: "-100%" }}
            animate={{ y: 0 }}
            exit={{ y: "-100%" }}
            transition={{ type: "spring", stiffness: 420, damping: 38 }}
            onMouseDown={(e) => e.stopPropagation()}
            onKeyDown={(e) => {
              if (e.key === "Escape") onClose();
              if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) submit();
            }}
            className="h-fit w-[560px] rounded-b-[14px] bg-surface p-6 shadow-[0_0_0_0.5px_var(--color-hairline),0_20px_50px_rgb(0_0_0/0.22)]"
          >
            <h2 className="text-headline font-semibold">Add a job</h2>
            <p className="mt-1 mb-5 text-callout text-ink-2">For jobs you find on LinkedIn, through a referral, or anywhere else. Paste the full description so Waypoint can score it.</p>
            <div className="grid grid-cols-2 gap-3">
              <Field label="Job title">
                <Input autoFocus value={form.title} onChange={set("title")} placeholder="Senior Product Designer" />
              </Field>
              <Field label="Company">
                <Input value={form.company} onChange={set("company")} placeholder="Acme" />
              </Field>
              <Field label="Location">
                <Input value={form.location} onChange={set("location")} placeholder="Remote, or Berlin" />
              </Field>
              <Field label="Link">
                <Input value={form.url} onChange={set("url")} placeholder="https://…" />
              </Field>
              <Field label="Job description" className="col-span-2">
                <Textarea value={form.description} onChange={set("description")} placeholder="Paste the posting here" className="min-h-40" />
              </Field>
            </div>
            <div className="mt-5 flex justify-end gap-2">
              <Button variant="plain" onClick={onClose}>Cancel</Button>
              <Button variant="primary" busy={busy} onClick={submit} disabled={!form.title.trim() || !form.company.trim()}>
                Save job
              </Button>
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
