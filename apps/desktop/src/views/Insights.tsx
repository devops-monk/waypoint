import { useQuery } from "@tanstack/react-query";
import clsx from "clsx";
import { useState } from "react";
import { Panel, Segmented } from "../components/ui";
import { api } from "../lib/api";
import { SOURCE_LABEL } from "../lib/format";
import type { Insights as Data } from "../lib/types";

// One series per chart, so one hue: the app's indigo, validated against both surfaces.
const BAR = "bg-indigo";

const STAGE: Record<Data["funnel"][number]["key"], string> = {
  saved: "Saved",
  applied: "Applied",
  heard_back: "Heard back",
  interview: "Interviewed",
  offer: "Offer",
  hired: "Hired",
};

const pct = (n: number, d: number) => (d ? Math.round((n / d) * 100) : 0);
const weekLabel = (iso: string) => new Date(`${iso}T12:00:00`).toLocaleDateString(undefined, { day: "numeric", month: "short" });

/** A hover/focus tooltip that sits above its mark. */
function Tip({ children }: { children: React.ReactNode }) {
  return (
    <span role="tooltip" className="pointer-events-none absolute bottom-full left-1/2 z-10 mb-1.5 -translate-x-1/2 rounded-[7px] bg-ink px-2 py-1 text-caption whitespace-nowrap text-window opacity-0 shadow-lg transition-opacity group-hover:opacity-100 group-focus:opacity-100">
      {children}
    </span>
  );
}

function Tile({ label, value, note }: { label: string; value: string; note?: string }) {
  return (
    <Panel className="p-4">
      <p className="text-callout text-ink-2">{label}</p>
      <p className="mt-1 text-[28px] leading-none font-semibold tracking-tight">{value}</p>
      {note && <p className="mt-1.5 text-caption text-ink-3">{note}</p>}
    </Panel>
  );
}

function Funnel({ d }: { d: Data }) {
  const max = Math.max(1, ...d.funnel.map((s) => s.count));
  return (
    <ul className="flex flex-col gap-2.5" aria-label="Application funnel">
      {d.funnel.map((s) => (
        <li key={s.key} className="grid grid-cols-[96px_1fr] items-center gap-3">
          <span className="text-callout text-ink-2">{STAGE[s.key]}</span>
          <span className="flex items-center gap-2">
            <span tabIndex={0} className="group relative block h-5 min-w-[3px] outline-none" style={{ width: `${(s.count / max) * 100}%` }}>
              <span className={clsx("block h-full rounded-r-[4px]", BAR, s.count === 0 && "opacity-0")} />
              <Tip>{STAGE[s.key]}: {s.count} {s.count === 1 ? "role" : "roles"}{s.key !== "saved" && s.key !== "applied" && d.applied ? `, ${pct(s.count, d.applied)}% of applications` : ""}</Tip>
            </span>
            <span className="tabular text-callout font-medium whitespace-nowrap">
              {s.count}
              {s.key !== "saved" && s.key !== "applied" && d.applied > 0 && <span className="ml-1.5 font-normal text-ink-3">{pct(s.count, d.applied)}%</span>}
            </span>
          </span>
        </li>
      ))}
    </ul>
  );
}

function Weekly({ d }: { d: Data }) {
  const max = Math.max(1, ...d.weeks.map((w) => w.applied));
  const peak = d.weeks.reduce((a, w, i) => (w.applied > d.weeks[a].applied ? i : a), 0);
  const H = 120;
  return (
    <div>
      <div className="flex h-[140px] items-end gap-1.5 border-b border-hairline" aria-label="Applications per week">
        {d.weeks.map((w, i) => (
          <div key={w.start} tabIndex={0} className="group relative flex h-full flex-1 flex-col items-center justify-end outline-none">
            {(i === peak || i === d.weeks.length - 1) && w.applied > 0 && <span className="tabular mb-1 text-caption text-ink-2">{w.applied}</span>}
            <span className={clsx("block w-full max-w-6 rounded-t-[4px]", BAR)} style={{ height: w.applied ? Math.max(3, (w.applied / max) * H) : 0 }} />
            <span className="absolute inset-0" />
            <Tip>Week of {weekLabel(w.start)}: {w.applied} applied</Tip>
          </div>
        ))}
      </div>
      <div className="mt-1.5 flex justify-between text-caption text-ink-3">
        <span>{weekLabel(d.weeks[0].start)}</span>
        <span>This week</span>
      </div>
    </div>
  );
}

function Sources({ d }: { d: Data }) {
  return (
    <ul className="flex flex-col gap-2.5" aria-label="Reply rate by source">
      {d.by_source.map((s) => {
        const rate = pct(s.heard_back, s.applied);
        return (
          <li key={s.source} className="grid grid-cols-[120px_1fr] items-center gap-3">
            <span className="truncate text-callout text-ink-2">{SOURCE_LABEL[s.source] ?? s.source}</span>
            <span className="flex items-center gap-2">
              <span className="relative block h-5 flex-1 rounded-[4px] bg-fill">
                <span tabIndex={0} className="group absolute inset-y-0 left-0 block min-w-[3px] outline-none" style={{ width: `${rate}%` }}>
                  <span className={clsx("block h-full rounded-r-[4px]", BAR, rate === 0 && "opacity-0")} />
                  <Tip>{SOURCE_LABEL[s.source] ?? s.source}: {s.heard_back} of {s.applied} replied</Tip>
                </span>
              </span>
              <span className="tabular w-24 text-right text-callout whitespace-nowrap">
                <span className="font-medium">{rate}%</span>
                <span className="ml-1.5 text-ink-3">{s.heard_back}/{s.applied}</span>
              </span>
            </span>
          </li>
        );
      })}
    </ul>
  );
}

function Tables({ d }: { d: Data }) {
  const th = "pb-2 text-left text-caption font-medium text-ink-3";
  const td = "border-t border-hairline py-1.5 text-callout";
  return (
    <div className="grid grid-cols-3 gap-6">
      <table className="tabular">
        <thead><tr><th className={th}>Stage</th><th className={clsx(th, "text-right")}>Roles</th></tr></thead>
        <tbody>{d.funnel.map((s) => <tr key={s.key}><td className={td}>{STAGE[s.key]}</td><td className={clsx(td, "text-right")}>{s.count}</td></tr>)}</tbody>
      </table>
      <table className="tabular">
        <thead><tr><th className={th}>Week of</th><th className={clsx(th, "text-right")}>Applied</th></tr></thead>
        <tbody>{d.weeks.map((w) => <tr key={w.start}><td className={td}>{weekLabel(w.start)}</td><td className={clsx(td, "text-right")}>{w.applied}</td></tr>)}</tbody>
      </table>
      <table className="tabular">
        <thead><tr><th className={th}>Source</th><th className={clsx(th, "text-right")}>Replied</th></tr></thead>
        <tbody>{d.by_source.map((s) => <tr key={s.source}><td className={td}>{SOURCE_LABEL[s.source] ?? s.source}</td><td className={clsx(td, "text-right")}>{s.heard_back} of {s.applied}</td></tr>)}</tbody>
      </table>
    </div>
  );
}

export function Insights() {
  const { data } = useQuery({ queryKey: ["insights"], queryFn: api.insights });
  const [view, setView] = useState<"charts" | "tables">("charts");
  if (!data) return null;
  if (data.applied === 0) {
    return (
      <div className="flex flex-1 items-center justify-center p-8">
        <div className="max-w-sm text-center">
          <p className="text-body font-medium">Insights start with your first application</p>
          <p className="mt-1 text-callout text-ink-3">Move a job to Applied and Waypoint starts measuring who replies, how fast, and which sources and fit scores pay off.</p>
        </div>
      </div>
    );
  }
  const rate = pct(data.heard_back, data.applied);
  const days = data.median_days_to_reply;
  return (
    <div className="scroll flex-1">
      <div className="mx-auto flex max-w-[1080px] flex-col gap-6 px-8 py-6">
        <div className="grid grid-cols-4 gap-3">
          <Tile label="Applications" value={String(data.applied)} note={`${data.active} still open`} />
          <Tile label="Reply rate" value={`${rate}%`} note={`${data.heard_back} of ${data.applied} heard back, rejections included`} />
          <Tile label="Typical wait for a reply" value={days == null ? "–" : `${days < 1 ? "<1" : Math.round(days)} ${Math.round(days) === 1 ? "day" : "days"}`} note={days == null ? "No replies yet" : "Median, from applying to first answer"} />
          <Tile label="Interviews" value={String(data.funnel.find((s) => s.key === "interview")?.count ?? 0)} note={`${pct(data.funnel.find((s) => s.key === "interview")?.count ?? 0, data.applied)}% of applications`} />
        </div>

        <div className="flex justify-end">
          <Segmented value={view} onChange={setView} options={[{ value: "charts", label: "Charts" }, { value: "tables", label: "Tables" }]} />
        </div>

        {view === "tables" ? (
          <Panel className="p-5"><Tables d={data} /></Panel>
        ) : (
          <>
            <div className="grid grid-cols-2 gap-4">
              <Panel className="p-5">
                <h3 className="text-callout font-semibold">How far your applications get</h3>
                <p className="mb-4 text-caption text-ink-3">Every role you've saved, and the furthest stage it reached.</p>
                <Funnel d={data} />
              </Panel>
              <Panel className="p-5">
                <h3 className="text-callout font-semibold">Applications per week</h3>
                <p className="mb-4 text-caption text-ink-3">The last 12 weeks. A steady pace beats bursts.</p>
                <Weekly d={data} />
              </Panel>
            </div>
            <Panel className="p-5">
              <h3 className="text-callout font-semibold">Who replies</h3>
              <p className="mb-4 text-caption text-ink-3">Share of applications that heard back, by where you found the role. Small numbers swing a lot.</p>
              <Sources d={data} />
            </Panel>
          </>
        )}

        <Panel className="p-5">
          <h3 className="text-callout font-semibold">Does your fit score predict replies?</h3>
          {data.fit ? (
            <p className="mt-2 text-body">
              Roles that replied scored <strong className="font-semibold">{Math.round(data.fit.heard_back_avg)}</strong> on average; roles that stayed silent scored{" "}
              <strong className="font-semibold">{Math.round(data.fit.silent_avg)}</strong>.{" "}
              <span className="text-ink-2">
                {data.fit.heard_back_avg - data.fit.silent_avg >= 8
                  ? "Your strongest matches are paying off, so lean towards them."
                  : data.fit.heard_back_avg - data.fit.silent_avg <= -8
                    ? "Lower-scored roles are replying more. Your profile may undersell something those employers value."
                    : "So far, score and replies aren't clearly linked."}
              </span>
              <span className="mt-1 block text-caption text-ink-3">Based on {data.fit.heard_back_n} scored roles that replied and {data.fit.silent_n} that didn't.</span>
            </p>
          ) : (
            <p className="mt-2 text-callout text-ink-3">This appears once at least three scored applications have replied and three haven't.</p>
          )}
        </Panel>
      </div>
    </div>
  );
}
