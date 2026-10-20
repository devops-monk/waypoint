import { useQuery, useQueryClient } from "@tanstack/react-query";
import clsx from "clsx";
import { AlertTriangle, Check, Plus, Search, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";
import { toast } from "sonner";
import { openUrl } from "@tauri-apps/plugin-opener";
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
              : "Waypoint reads Greenhouse, Lever, Ashby, Workday, SmartRecruiters, Workable, Recruitee, Personio and Teamtailor boards. This link isn't one of them yet."
            : "Paste the company's careers page link, e.g. jobs.ashbyhq.com/acme or acme.wd5.myworkdayjobs.com/External"
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

const ADZUNA_COUNTRIES: [string, string][] = [
  ["us", "United States"], ["gb", "United Kingdom"], ["in", "India"], ["de", "Germany"], ["nl", "Netherlands"], ["fr", "France"],
  ["at", "Austria"], ["be", "Belgium"], ["ch", "Switzerland"], ["es", "Spain"], ["it", "Italy"], ["pl", "Poland"], ["ca", "Canada"],
  ["au", "Australia"], ["nz", "New Zealand"], ["sg", "Singapore"], ["za", "South Africa"], ["br", "Brazil"], ["mx", "Mexico"],
];

/** Sources whose terms ask for a visible link back, which every saved job keeps. */
const ATTRIBUTION: Record<string, string> = {
  remotive: "Jobs link back to Remotive, as their terms require.",
  remoteok: "Jobs link back to Remote OK, as their terms require.",
};

function AdzunaKeyForm({ onSaved }: { onSaved: () => void }) {
  const [appId, setAppId] = useState("");
  const [appKey, setAppKey] = useState("");
  const [busy, setBusy] = useState(false);
  const save = async () => {
    setBusy(true);
    try {
      await api.saveAdzunaKey(appId, appKey);
      toast.success("Adzuna key saved to your keychain");
      onSaved();
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="rounded-[10px] bg-fill p-3.5">
      <p className="text-callout font-medium">Adzuna needs your own free API key</p>
      <p className="mt-0.5 text-callout text-ink-2">
        Sign up at{" "}
        <button className="text-selection hover:underline" onClick={() => openUrl("https://developer.adzuna.com/signup")}>developer.adzuna.com</button>
        , then copy the app ID and key from your dashboard. They're stored in your system keychain.
      </p>
      <div className="mt-3 grid grid-cols-[1fr_1fr_auto] items-end gap-3">
        <Field label="App ID"><Input value={appId} onChange={(e) => setAppId(e.target.value)} autoComplete="off" spellCheck={false} /></Field>
        <Field label="App key"><Input type="password" value={appKey} onChange={(e) => setAppKey(e.target.value)} autoComplete="off" spellCheck={false} /></Field>
        <Button busy={busy} disabled={!appId.trim() || !appKey.trim()} onClick={save}>Save key</Button>
      </div>
    </div>
  );
}

function AddSearch({ onAdded }: { onAdded: () => void }) {
  const qc = useQueryClient();
  const { data: providers = [] } = useQuery({ queryKey: ["providers"], queryFn: api.providers });
  const { data: keys } = useQuery({ queryKey: ["source-keys"], queryFn: api.sourceKeyStatus });
  const searches = providers.filter((p) => p.kind === "search");
  const [id, setId] = useState("remotive");
  const [q, setQ] = useState("");
  const [place, setPlace] = useState("");
  const [country, setCountry] = useState("in");
  const provider = searches.find((p) => p.id === id);
  const needsKey = provider?.needs_key === "Adzuna" && keys && !keys.adzuna;

  const add = async () => {
    if (!provider || !q.trim()) return;
    const what = q.trim();
    const where = provider.place ? place.trim() : "";
    const query = (provider.id === "adzuna" ? `${country}:` : "") + what + (where ? ` @ ${where}` : "");
    const countryName = ADZUNA_COUNTRIES.find(([c]) => c === country)?.[1];
    const scope = [where, provider.id === "adzuna" ? countryName : null].filter(Boolean).join(", ");
    const label = `${provider.name}: ${what}${scope ? ` in ${scope}` : ""}`;
    try {
      await api.addBoard(provider.id, query, label);
      toast.success(`Watching ${label}`);
      setQ("");
      setPlace("");
      onAdded();
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap gap-1.5" role="radiogroup" aria-label="Job search source">
        {searches.map((p) => (
          <button
            key={p.id}
            role="radio"
            aria-checked={p.id === id}
            onClick={() => setId(p.id)}
            className={clsx(
              "h-7 rounded-full px-3 text-callout font-medium transition-colors",
              p.id === id ? "bg-selection text-white" : "bg-fill text-ink-2 hover:bg-fill-strong hover:text-ink",
            )}
          >
            {p.name}
          </button>
        ))}
      </div>
      {provider && (
        <>
          <p className="-mt-1 text-callout text-ink-2">{provider.coverage}.{ATTRIBUTION[provider.id] ? ` ${ATTRIBUTION[provider.id]}` : ""}</p>
          {needsKey ? (
            <AdzunaKeyForm onSaved={() => qc.invalidateQueries({ queryKey: ["source-keys"] })} />
          ) : (
            <div className={clsx("grid items-end gap-3", provider.id === "adzuna" ? "grid-cols-[160px_1fr_180px_auto]" : provider.place ? "grid-cols-[1fr_200px_auto]" : "grid-cols-[1fr_auto]")}>
              {provider.id === "adzuna" && (
                <Field label="Country">
                  <select value={country} onChange={(e) => setCountry(e.target.value)} className="h-8 rounded-control bg-surface px-2 text-body text-ink shadow-[0_0_0_0.5px_var(--color-hairline)] outline-none">
                    {ADZUNA_COUNTRIES.map(([c, n]) => <option key={c} value={c}>{n}</option>)}
                  </select>
                </Field>
              )}
              <Field label="Keywords">
                <Input value={q} onChange={(e) => setQ(e.target.value.replace("@", ""))} placeholder={provider.example.split("@")[0].replace(/^[a-z]{2}:/, "").trim()} onKeyDown={(e) => e.key === "Enter" && add()} />
              </Field>
              {provider.place && (
                <Field label="Place (optional)">
                  <Input value={place} onChange={(e) => setPlace(e.target.value.replace("@", ""))} placeholder={provider.example.split("@")[1]?.trim() ?? ""} onKeyDown={(e) => e.key === "Enter" && add()} />
                </Field>
              )}
              <Button disabled={!q.trim()} onClick={add}>Watch</Button>
            </div>
          )}
          {provider.id === "adzuna" && keys?.adzuna && (
            <button
              className="-mt-2 self-start text-caption text-ink-3 hover:text-coral"
              onClick={async () => {
                await api.deleteAdzunaKey();
                qc.invalidateQueries({ queryKey: ["source-keys"] });
                toast("Adzuna key removed");
              }}
            >
              Remove Adzuna key
            </button>
          )}
        </>
      )}
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
                <h2 className="text-headline font-semibold">{mode === "link" ? "Follow a company" : "Watch a job search"}</h2>
                <p className="mt-0.5 text-callout text-ink-2">
                  {mode === "link" ? "Waypoint reads public job boards directly. It never logs in or scrapes LinkedIn." : "New matches show up in Discover each time you check for jobs."}
                </p>
              </div>
              <Segmented value={mode} onChange={setMode} options={[{ value: "link", label: "Careers page" }, { value: "search", label: "Job search" }]} />
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
