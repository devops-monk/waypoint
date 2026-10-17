import { useQuery } from "@tanstack/react-query";
import { Command } from "cmdk";
import { AnimatePresence, motion } from "motion/react";
import { Building2, FileText, Inbox, Plus, RefreshCw, Settings2, SquareKanban, UserRound } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { api, errorMessage } from "../lib/api";
import { STATUS_LABEL } from "../lib/format";
import type { View } from "./Sidebar";
import { FitBadge } from "./FitDial";

const item = "flex h-9 cursor-default items-center gap-2.5 rounded-[7px] px-2.5 text-body text-ink data-[selected=true]:bg-selection data-[selected=true]:text-white";

export function CommandPalette({ open, onClose, onNavigate, onOpenJob, onAdd }: { open: boolean; onClose: () => void; onNavigate: (v: View) => void; onOpenJob: (id: number) => void; onAdd: () => void }) {
  const [search, setSearch] = useState("");
  const { data: jobs } = useQuery({
    queryKey: ["palette-jobs", search],
    queryFn: () => api.listJobs({ query: search, limit: 8 }),
    enabled: open && search.trim().length > 1,
  });

  const run = (fn: () => void) => {
    fn();
    setSearch("");
    onClose();
  };

  const scan = () =>
    toast.promise(api.scanAll(), {
      loading: "Checking your companies for new jobs…",
      success: (s) => (s.new_jobs ? `Found ${s.new_jobs} new ${s.new_jobs === 1 ? "job" : "jobs"}` : "No new jobs since the last check"),
      error: (e) => errorMessage(e),
    });

  return (
    <AnimatePresence>
      {open && (
        <motion.div className="fixed inset-0 z-50 flex items-start justify-center bg-black/15 pt-[14vh] dark:bg-black/35" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.12 }} onMouseDown={onClose}>
          <motion.div
            initial={{ opacity: 0, scale: 0.97, y: -6 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.98 }}
            transition={{ type: "spring", stiffness: 500, damping: 34 }}
            onMouseDown={(e) => e.stopPropagation()}
            className="w-[580px] overflow-hidden rounded-[14px] bg-surface/95 shadow-[0_0_0_0.5px_var(--color-hairline),0_24px_60px_rgb(0_0_0/0.25)] backdrop-blur-2xl"
          >
            <Command label="Command menu" shouldFilter={!jobs?.length} onKeyDown={(e) => e.key === "Escape" && onClose()}>
              <Command.Input autoFocus value={search} onValueChange={setSearch} placeholder="Search jobs, or type a command" className="h-12 w-full border-b border-hairline bg-transparent px-4 text-headline text-ink outline-none placeholder:text-ink-3" />
              <Command.List className="scroll max-h-[360px] p-1.5">
                <Command.Empty className="px-3 py-6 text-center text-callout text-ink-3">No matches</Command.Empty>
                {jobs && jobs.length > 0 && (
                  <Command.Group heading="Jobs" className="[&_[cmdk-group-heading]]:px-2.5 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-caption [&_[cmdk-group-heading]]:text-ink-3">
                    {jobs.map((j) => (
                      <Command.Item key={j.id} value={`job-${j.id}-${j.title}-${j.company}`} onSelect={() => run(() => onOpenJob(j.id))} className={item}>
                        <span className="min-w-0 flex-1 truncate">
                          {j.title} <span className="opacity-60">at {j.company}</span>
                        </span>
                        <span className="text-caption opacity-60">{STATUS_LABEL[j.status]}</span>
                        <FitBadge score={j.score} band={j.band} />
                      </Command.Item>
                    ))}
                  </Command.Group>
                )}
                <Command.Group heading="Go to" className="[&_[cmdk-group-heading]]:px-2.5 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-caption [&_[cmdk-group-heading]]:text-ink-3">
                  <Command.Item onSelect={() => run(() => onNavigate("discover"))} className={item}><Inbox className="size-4" />Discover</Command.Item>
                  <Command.Item onSelect={() => run(() => onNavigate("pipeline"))} className={item}><SquareKanban className="size-4" />Pipeline</Command.Item>
                  <Command.Item onSelect={() => run(() => onNavigate("resume"))} className={item}><FileText className="size-4" />Resume</Command.Item>
                  <Command.Item onSelect={() => run(() => onNavigate("sources"))} className={item}><Building2 className="size-4" />Companies</Command.Item>
                  <Command.Item onSelect={() => run(() => onNavigate("profile"))} className={item}><UserRound className="size-4" />Profile</Command.Item>
                  <Command.Item onSelect={() => run(() => onNavigate("settings"))} className={item}><Settings2 className="size-4" />Settings</Command.Item>
                </Command.Group>
                <Command.Group heading="Actions" className="[&_[cmdk-group-heading]]:px-2.5 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-caption [&_[cmdk-group-heading]]:text-ink-3">
                  <Command.Item onSelect={() => run(scan)} className={item}><RefreshCw className="size-4" />Check for new jobs</Command.Item>
                  <Command.Item onSelect={() => run(onAdd)} className={item}><Plus className="size-4" />Add a job</Command.Item>
                </Command.Group>
              </Command.List>
            </Command>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
