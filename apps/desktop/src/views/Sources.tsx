import { useQuery, useQueryClient } from "@tanstack/react-query";
import clsx from "clsx";
import { AlertTriangle, Check, Plus, Search, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";
import { toast } from "sonner";
import { Button, Field, IconButton, Input, Panel, Segmented, Toggle } from "../components/ui";
import { api, errorMessage } from "../lib/api";
import { SOURCE_LABEL, relativeTime } from "../lib/format";
import type { Board } from "../lib/types";

function boardHealth(b: Board): { tone: "ok" | "warn" | "error" | "idle"; text: string } {
  if (b.last_error) return { tone: "error", text: b.last_error.includes("404") ? "Board not found. The company may have moved to another hiring system." : `Couldn't reach it: ${b.last_error}` };
  if (b.last_run_at == null) return { tone: "idle", text: "Not checked yet" };
  if (b.last_count === 0) return { tone: "warn", text: "No open roles. If this lasts, the company may have moved its job board." };
  return { tone: "ok", text: `${b.last_count} open ${b.last_count === 1 ? "role" : "roles"}, checked ${relativeTime(b.last_run_at).toLowerCase()}` };
}

function AddByLink({ onAdded }: { onAdded: () => void }) {
  const [url, setUrl] = useState("");
  const [company, setCompany] = useState("");
  const [busy, setBusy] = useState(false);
  const { data: detected } = useQuery({ queryKey: ["detect", url], queryFn: () => api.detectBoard(url), enabled: url.startsWith("https://") });

  const add = async () => {
    if (!detected) return;
    setBusy(true);
    try {
      await api.addBoard(detected.provider, detected.slug, company);
      toast.success(`Following ${company || detected.slug}`);
      setUrl("");
      setCompany("");
      onAdded();
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="grid grid-cols-[1fr_200px_auto] items-end gap-3">
      <Field
        label="Careers page link"
        hint={
          url.startsWith("https://")
            ? detected
              ? <span className="text-teal">Found a {SOURCE_LABEL[detected.provider]} job board</span>
              : "Waypoint reads Greenhouse, Lever and Ashby job boards. This link isn't one of them yet."
            : "Paste a link like job-boards.greenhouse.io/acme, jobs.lever.co/acme or jobs.ashbyhq.com/acme"
        }
      >
        <Input value={url} onChange={(e) => setUrl(e.target.value.trim())} placeholder="https://jobs.ashbyhq.com/…" />
      </Field>
      <Field label="Company name">
        <Input value={company} onChange={(e) => setCompany(e.target.value)} placeholder={detected?.slug ?? "Acme"} />
      </Field>
      <Button variant="primary" className="mb-[18px]" busy={busy} disabled={!detected} onClick={add}>
        Follow
      </Button>
    </div>
  );
}

function AddSearch({ onAdded }: { onAdded: () => void }) {
  const [q, setQ] = useState("");
  const add = async () => {
    try {
      await api.addBoard("remotive", q.trim(), `Remote: ${q.trim()}`);
      toast.success(`Watching remote jobs for “${q.trim()}”`);
      setQ("");
      onAdded();
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };
  return (
    <div className="grid grid-cols-[1fr_auto] items-end gap-3">
      <Field label="Remote jobs keyword" hint="Searches Remotive's remote job board. Jobs link back to Remotive, as their terms require.">
        <Input value={q} onChange={(e) => setQ(e.target.value)} placeholder="rust, product designer, data engineer" onKeyDown={(e) => e.key === "Enter" && q.trim() && add()} />
      </Field>
      <Button className="mb-[34px]" disabled={!q.trim()} onClick={add}>
        Watch
      </Button>
    </div>
  );
}

export function Sources() {
  const qc = useQueryClient();
  const { data: boards = [] } = useQuery({ queryKey: ["boards"], queryFn: api.listBoards });
  const { data: seeds = [] } = useQuery({ queryKey: ["seeds"], queryFn: api.seedBoards });
  const [mode, setMode] = useState<"link" | "search">("link");
  const [filter, setFilter] = useState("");
  const refresh = () => qc.invalidateQueries({ queryKey: ["boards"] });

  const followed = useMemo(() => new Set(boards.map((b) => `${b.provider}:${b.slug}`)), [boards]);
  const visibleSeeds = seeds.filter((s) => s.company.toLowerCase().includes(filter.toLowerCase()));

  const follow = async (provider: string, slug: string, company: string) => {
    try {
      await api.addBoard(provider, slug, company);
      refresh();
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="drag flex h-[52px] shrink-0 items-center gap-3 border-b border-hairline px-4" data-tauri-drag-region>
        <h1 className="text-headline font-semibold" data-tauri-drag-region>Companies</h1>
        <span className="text-callout text-ink-3" data-tauri-drag-region>Following {boards.filter((b) => b.enabled).length}</span>
      </header>
      <div className="scroll flex-1">
        <div className="mx-auto flex max-w-[860px] flex-col gap-8 px-8 py-7">
          <section>
            <div className="mb-3 flex items-center justify-between">
              <div>
                <h2 className="text-headline font-semibold">Follow a company</h2>
                <p className="mt-0.5 text-callout text-ink-2">Waypoint reads public job boards directly. It never logs in or scrapes LinkedIn.</p>
              </div>
              <Segmented value={mode} onChange={setMode} options={[{ value: "link", label: "Careers page" }, { value: "search", label: "Remote keyword" }]} />
            </div>
            <Panel className="p-4">{mode === "link" ? <AddByLink onAdded={refresh} /> : <AddSearch onAdded={refresh} />}</Panel>
          </section>

          {boards.length > 0 && (
            <section>
              <h2 className="mb-3 text-headline font-semibold">Following</h2>
              <Panel className="divide-y divide-hairline">
                {boards.map((b) => {
                  const h = boardHealth(b);
                  return (
                    <div key={b.id} className={clsx("flex items-center gap-3 px-4 py-2.5", !b.enabled && "opacity-55")}>
                      <div className="min-w-0 flex-1">
                        <p className="truncate text-body font-medium">{b.company}</p>
                        <p className={clsx("flex items-center gap-1.5 truncate text-caption", h.tone === "error" ? "text-coral" : h.tone === "warn" ? "text-amber" : "text-ink-3")}>
                          {(h.tone === "error" || h.tone === "warn") && <AlertTriangle className="size-3 shrink-0" />}
                          <span className="truncate">{h.text}</span>
                        </p>
                      </div>
                      <span className="text-caption text-ink-3">{SOURCE_LABEL[b.provider]}</span>
                      <Toggle
                        checked={b.enabled}
                        label={b.enabled ? `Pause ${b.company}` : `Resume ${b.company}`}
                        onChange={async (v) => {
                          await api.setBoardEnabled(b.id, v);
                          refresh();
                        }}
                      />
                      <IconButton
                        label={`Stop following ${b.company}`}
                        onClick={async () => {
                          await api.removeBoard(b.id);
                          refresh();
                        }}
                      >
                        <Trash2 className="size-3.5" />
                      </IconButton>
                    </div>
                  );
                })}
              </Panel>
            </section>
          )}

          <section>
            <div className="mb-3 flex items-end justify-between gap-4">
              <div>
                <h2 className="text-headline font-semibold">Suggestions</h2>
                <p className="mt-0.5 text-callout text-ink-2">Companies with public job boards, mostly in tech. Follow the ones you'd work for.</p>
              </div>
              <div className="relative w-56">
                <Search className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-ink-3" />
                <Input value={filter} onChange={(e) => setFilter(e.target.value)} placeholder="Find a company" className="h-7 pl-7 text-callout" />
              </div>
            </div>
            <div className="grid grid-cols-3 gap-2">
              {visibleSeeds.map((s) => {
                const on = followed.has(`${s.provider}:${s.slug}`);
                return (
                  <button
                    key={`${s.provider}:${s.slug}`}
                    disabled={on}
                    onClick={() => follow(s.provider, s.slug, s.company)}
                    className={clsx(
                      "flex items-center gap-2 rounded-[9px] px-3 py-2 text-left transition-colors",
                      on ? "bg-teal/10" : "bg-surface shadow-[0_0_0_0.5px_var(--color-hairline)] hover:bg-raised",
                    )}
                  >
                    <span className="min-w-0 flex-1 truncate text-callout font-medium">{s.company}</span>
                    {on ? <Check className="size-3.5 text-teal" /> : <Plus className="size-3.5 text-ink-3" />}
                  </button>
                );
              })}
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
