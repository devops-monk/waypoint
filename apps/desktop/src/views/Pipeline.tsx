import { DndContext, DragOverlay, PointerSensor, useDraggable, useDroppable, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import clsx from "clsx";
import { useState } from "react";
import { toast } from "sonner";
import { FitBadge } from "../components/FitDial";
import { Segmented } from "../components/ui";
import { FollowUps } from "./FollowUps";
import { api, errorMessage } from "../lib/api";
import { EXITS, ROUTE, STATUS_LABEL } from "../lib/format";
import type { JobSummary, Status } from "../lib/types";

const COLUMNS: Status[] = ["saved", "evaluated", "applied", "responded", "assessment", "interview", "offer", "hired"];

function Card({ job, onOpen, overlay, due }: { job: JobSummary; onOpen?: () => void; overlay?: boolean; due?: string }) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({ id: job.id, data: job, disabled: overlay });
  return (
    <div
      ref={overlay ? undefined : setNodeRef}
      {...(overlay ? {} : listeners)}
      {...(overlay ? {} : attributes)}
      onDoubleClick={onOpen}
      onKeyDown={(e) => e.key === "Enter" && onOpen?.()}
      className={clsx(
        "rounded-[10px] bg-surface p-3 shadow-[0_0_0_0.5px_var(--color-hairline),0_1px_2px_rgb(0_0_0/0.05)] transition-shadow",
        isDragging && "opacity-30",
        overlay && "rotate-[1.5deg] shadow-[0_0_0_0.5px_var(--color-hairline),0_14px_30px_rgb(0_0_0/0.2)]",
      )}
    >
      <div className="flex items-start gap-2">
        <div className="min-w-0 flex-1">
          <p className="truncate text-caption font-medium text-ink-2">{job.company}</p>
          <p className="mt-0.5 line-clamp-2 text-callout leading-snug font-semibold text-ink">{job.title}</p>
        </div>
        <FitBadge score={job.score} band={job.band} />
      </div>
      <p className="mt-1.5 truncate text-caption text-ink-3">{job.location || "Location not listed"}</p>
      {due && (
        <p className="mt-1.5 flex items-center gap-1.5 text-caption font-medium text-amber">
          <span className="size-1.5 rounded-full bg-amber" />
          {due}
        </p>
      )}
    </div>
  );
}

function Column({ status, jobs, onOpen, due }: { status: Status; jobs: JobSummary[]; onOpen: (id: number) => void; due: Map<number, string> }) {
  const { setNodeRef, isOver } = useDroppable({ id: status });
  const onRoute = ROUTE.includes(status);
  return (
    <section ref={setNodeRef} className={clsx("flex w-[248px] shrink-0 flex-col rounded-[12px] transition-colors", isOver ? "bg-selection/8" : "bg-transparent")}>
      <header className="flex items-center gap-2 px-2 pt-1 pb-3">
        <span className={clsx("size-2 rounded-full", jobs.length ? "bg-teal" : onRoute ? "bg-fill-strong" : "bg-ink-3")} />
        <h2 className="text-callout font-semibold">{STATUS_LABEL[status]}</h2>
        <span className="tabular text-callout text-ink-3">{jobs.length || ""}</span>
        {onRoute && status !== "hired" && <span aria-hidden className="-mr-5 ml-1 h-0 flex-1 border-t-2 border-dotted border-fill-strong" />}
      </header>
      <div className="scroll flex min-h-24 flex-1 flex-col gap-2 px-1 pb-4">
        {jobs.map((j) => (
          <Card key={j.id} job={j} onOpen={() => onOpen(j.id)} due={due.get(j.id)} />
        ))}
        {jobs.length === 0 && <div className="rounded-[10px] border border-dashed border-fill-strong px-3 py-5 text-center text-caption text-ink-3">Drop a job here</div>}
      </div>
    </section>
  );
}

export function Pipeline({ onOpenJob }: { onOpenJob: (id: number) => void }) {
  const qc = useQueryClient();
  const { data: jobs = [] } = useQuery({ queryKey: ["jobs", "pipeline"], queryFn: () => api.listJobs({ statuses: [...COLUMNS, ...EXITS], limit: 2000 }) });
  const { data: followups } = useQuery({ queryKey: ["followups"], queryFn: api.followups, refetchInterval: 60_000 });
  const [tab, setTab] = useState<"board" | "followups">("board");
  const [dragging, setDragging] = useState<JobSummary | null>(null);
  const due = new Map(
    (followups?.items ?? [])
      .filter((i) => i.urgency === "urgent" || i.urgency === "due")
      .map((i) => [i.job_id, i.action === "reply" ? "Reply due" : i.action === "thank_you" ? "Thank-you due" : "Follow-up due"] as const),
  );
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 4 } }));

  const byStatus = (s: Status) => jobs.filter((j) => j.status === s);
  const closed = jobs.filter((j) => EXITS.includes(j.status));

  const onDragEnd = async (e: DragEndEvent) => {
    setDragging(null);
    const to = e.over?.id as Status | undefined;
    const job = e.active.data.current as JobSummary | undefined;
    if (!to || !job || job.status === to) return;
    // Optimistic move, then persist.
    qc.setQueryData<JobSummary[]>(["jobs", "pipeline"], (old) => old?.map((j) => (j.id === job.id ? { ...j, status: to } : j)));
    try {
      await api.setStatus(job.id, to);
      if (to === "applied") toast.success(`Moved to Applied: ${job.company}`);
    } catch (err) {
      toast.error(errorMessage(err));
    }
    qc.invalidateQueries();
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="drag flex h-[52px] shrink-0 items-center gap-3 border-b border-hairline px-4" data-tauri-drag-region>
        <h1 className="text-headline font-semibold" data-tauri-drag-region>Pipeline</h1>
        <span className="text-callout text-ink-3" data-tauri-drag-region>
          {jobs.length - closed.length} active{closed.length ? `, ${closed.length} closed` : ""}
        </span>
        <div className="flex-1" data-tauri-drag-region />
        <Segmented value={tab} onChange={setTab} options={[{ value: "board", label: "Board" }, { value: "followups", label: due.size ? `Follow-ups (${due.size})` : "Follow-ups" }]} />
      </header>
      {tab === "followups" ? (
        <FollowUps onOpenJob={onOpenJob} />
      ) : (
      <>
      <DndContext sensors={sensors} onDragStart={(e) => setDragging(e.active.data.current as JobSummary)} onDragEnd={onDragEnd} onDragCancel={() => setDragging(null)}>
        <div className="scroll flex min-h-0 flex-1 gap-3 overflow-x-auto p-4">
          {COLUMNS.map((s) => (
            <Column key={s} status={s} jobs={byStatus(s)} onOpen={onOpenJob} due={due} />
          ))}
          <div className="mx-1 w-px shrink-0 self-stretch bg-hairline" />
          {EXITS.map((s) => (
            <Column key={s} status={s} jobs={byStatus(s)} onOpen={onOpenJob} due={due} />
          ))}
        </div>
        <DragOverlay dropAnimation={{ duration: 180, easing: "cubic-bezier(0.32, 0.72, 0, 1)" }}>{dragging && <Card job={dragging} overlay />}</DragOverlay>
      </DndContext>
      <p className="shrink-0 border-t border-hairline px-4 py-2 text-caption text-ink-3">Drag cards between stages. Double-click a card to open it.</p>
      </>
      )}
    </div>
  );
}
