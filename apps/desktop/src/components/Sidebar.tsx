import { useQuery } from "@tanstack/react-query";
import clsx from "clsx";
import { Building2, Inbox, Plus, Search, Settings2, SquareKanban, UserRound } from "lucide-react";
import type { ReactNode } from "react";
import { api } from "../lib/api";
import { EXITS } from "../lib/format";
import { Kbd } from "./ui";

export type View = "discover" | "pipeline" | "sources" | "profile" | "settings";

function NavItem({ active, icon, label, count, onClick }: { active: boolean; icon: ReactNode; label: string; count?: number; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      aria-current={active ? "page" : undefined}
      className={clsx(
        "flex h-7 w-full items-center gap-2.5 rounded-[6px] px-2 text-body transition-colors",
        active ? "bg-fill-strong font-medium text-ink" : "text-ink-2 hover:bg-fill hover:text-ink",
      )}
    >
      <span className={clsx("flex size-4 items-center justify-center", active ? "text-selection" : "text-ink-3")}>{icon}</span>
      <span className="flex-1 text-left">{label}</span>
      {count != null && count > 0 && <span className="tabular text-callout text-ink-3">{count}</span>}
    </button>
  );
}

export function Sidebar({ view, onNavigate, onAdd, onSearch }: { view: View; onNavigate: (v: View) => void; onAdd: () => void; onSearch: () => void }) {
  const { data: counts } = useQuery({ queryKey: ["pipeline-counts"], queryFn: api.pipelineCounts, refetchInterval: 30_000 });
  const count = (pred: (s: string) => boolean) => counts?.filter((c) => pred(c.status)).reduce((a, c) => a + c.count, 0) ?? 0;
  const inbox = count((s) => s === "new");
  const active = count((s) => s !== "new" && !EXITS.includes(s as never));

  return (
    <aside className="flex w-[232px] shrink-0 flex-col bg-sidebar">
      {/* Title-bar strip: room for the traffic lights, and a handle to drag the window. */}
      <div className="drag h-[52px] shrink-0" data-tauri-drag-region />
      <div className="px-3">
        <button
          onClick={onSearch}
          className="mb-4 flex h-7 w-full items-center gap-2 rounded-[7px] bg-fill px-2 text-callout text-ink-3 transition-colors hover:bg-fill-strong"
        >
          <Search className="size-3.5" />
          <span className="flex-1 text-left">Search</span>
          <Kbd>⌘K</Kbd>
        </button>
        <nav className="flex flex-col gap-0.5">
          <NavItem active={view === "discover"} icon={<Inbox className="size-4" />} label="Discover" count={inbox} onClick={() => onNavigate("discover")} />
          <NavItem active={view === "pipeline"} icon={<SquareKanban className="size-4" />} label="Pipeline" count={active} onClick={() => onNavigate("pipeline")} />
          <NavItem active={view === "sources"} icon={<Building2 className="size-4" />} label="Companies" onClick={() => onNavigate("sources")} />
        </nav>
        <p className="mt-5 mb-1 px-2 text-caption font-medium text-ink-3">You</p>
        <nav className="flex flex-col gap-0.5">
          <NavItem active={view === "profile"} icon={<UserRound className="size-4" />} label="Profile" onClick={() => onNavigate("profile")} />
          <NavItem active={view === "settings"} icon={<Settings2 className="size-4" />} label="Settings" onClick={() => onNavigate("settings")} />
        </nav>
      </div>
      <div className="mt-auto p-3">
        <button
          onClick={onAdd}
          className="flex h-8 w-full items-center gap-2 rounded-[7px] px-2 text-callout font-medium text-ink-2 transition-colors hover:bg-fill hover:text-ink"
        >
          <Plus className="size-4" />
          <span className="flex-1 text-left">Add a job</span>
          <Kbd>⌘N</Kbd>
        </button>
      </div>
    </aside>
  );
}
