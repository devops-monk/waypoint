import { useQuery, useQueryClient } from "@tanstack/react-query";
import { open as openFile, save } from "@tauri-apps/plugin-dialog";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import clsx from "clsx";
import { Check, KeyRound, ShieldCheck } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
import type { Update } from "@tauri-apps/plugin-updater";
import { findUpdate, getVersion, installUpdate } from "../lib/updates";
import { Button, Field, Input, Panel, Segmented, Toggle } from "../components/ui";
import { api, errorMessage } from "../lib/api";
import type { AiProvider, Cadence, FollowUpSettings, LlmConfig } from "../lib/types";

const PROVIDERS: { id: AiProvider; name: string; blurb: string; models: string[]; keyUrl?: string }[] = [
  { id: "anthropic", name: "Claude", blurb: "Anthropic's models. Strongest at careful, honest evaluation.", models: ["claude-opus-5-5", "claude-sonnet-5-5", "claude-haiku-5-5"], keyUrl: "https://console.anthropic.com/settings/keys" },
  { id: "openai", name: "OpenAI", blurb: "GPT models through your OpenAI account.", models: ["gpt-5", "gpt-5-mini"], keyUrl: "https://platform.openai.com/api-keys" },
  { id: "openrouter", name: "OpenRouter", blurb: "One key for many models, including free ones.", models: ["anthropic/claude-opus-5-5", "anthropic/claude-sonnet-5-5", "openai/gpt-5"], keyUrl: "https://openrouter.ai/keys" },
  { id: "ollama", name: "Ollama", blurb: "Free and fully private: models run on this computer.", models: ["llama3.3", "qwen3", "gemma3"] },
];

function UsagePanel() {
  const { data: usage = [] } = useQuery({ queryKey: ["usage"], queryFn: api.tokenUsage });
  if (!usage.length) return <p className="text-callout text-ink-3">No AI calls yet. Token use appears here after your first AI call.</p>;
  return (
    <table className="w-full text-callout">
      <thead>
        <tr className="text-left text-caption text-ink-3">
          <th className="pb-2 font-medium">Model</th>
          <th className="pb-2 text-right font-medium">AI calls</th>
          <th className="pb-2 text-right font-medium">Tokens in</th>
          <th className="pb-2 text-right font-medium">Tokens out</th>
        </tr>
      </thead>
      <tbody className="tabular">
        {usage.map((u) => (
          <tr key={u.model} className="border-t border-hairline">
            <td className="py-2">{u.model}</td>
            <td className="py-2 text-right">{u.calls}</td>
            <td className="py-2 text-right">{u.input_tokens.toLocaleString()}</td>
            <td className="py-2 text-right">{u.output_tokens.toLocaleString()}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

const CADENCE_FIELDS: { key: keyof Cadence; label: string; unit: string }[] = [
  { key: "applied_first", label: "First follow-up after applying", unit: "days" },
  { key: "applied_subsequent", label: "Next follow-up after that", unit: "days" },
  { key: "applied_max", label: "Stop after", unit: "follow-ups" },
  { key: "responded_initial", label: "Reply to a recruiter within", unit: "days" },
  { key: "responded_subsequent", label: "Then keep in touch every", unit: "days" },
  { key: "interview_thankyou", label: "Thank-you note after an interview", unit: "days" },
];

function BackupPanel() {
  const [busy, setBusy] = useState<"export" | "restore" | null>(null);
  const [pending, setPending] = useState<{ path: string; jobs: number; applied: number; name: string } | null>(null);

  const exportBackup = async () => {
    const day = new Date().toISOString().slice(0, 10);
    const path = await save({ defaultPath: `Waypoint backup ${day}.waypoint`, filters: [{ name: "Waypoint backup", extensions: ["waypoint"] }] });
    if (!path) return;
    setBusy("export");
    try {
      await api.backupExport(path);
      toast.success("Backup saved", { action: { label: "Show file", onClick: () => revealItemInDir(path) } });
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
    }
  };

  const chooseRestore = async () => {
    const path = await openFile({ multiple: false, filters: [{ name: "Waypoint backup", extensions: ["waypoint", "db"] }] });
    if (!path || Array.isArray(path)) return;
    try {
      setPending({ path, ...(await api.backupInspect(path)) });
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };

  return (
    <Panel className="p-4">
      <div className="flex items-center gap-3">
        <div className="flex-1">
          <p className="text-body font-medium">Back up everything</p>
          <p className="text-callout text-ink-3">Jobs, your profile, every document and note, stories and follow-ups, in one file. API keys stay in your keychain and aren't included.</p>
        </div>
        <Button busy={busy === "export"} onClick={exportBackup}>Back up…</Button>
        <Button variant="plain" onClick={chooseRestore}>Restore…</Button>
      </div>
      {pending && (
        <div className="mt-3 rounded-[10px] bg-amber/10 p-3">
          <p className="text-callout font-medium">
            Restore the backup{pending.name ? ` of ${pending.name}` : ""}: {pending.jobs} {pending.jobs === 1 ? "job" : "jobs"}, {pending.applied} applied or further?
          </p>
          <p className="mt-0.5 text-callout text-ink-2">This replaces what's in Waypoint now. Your current data is kept as a dated copy in the app's data folder, and Waypoint restarts.</p>
          <div className="mt-2.5 flex justify-end gap-2">
            <Button size="sm" variant="plain" onClick={() => setPending(null)}>Cancel</Button>
            <Button
              size="sm"
              variant="primary"
              busy={busy === "restore"}
              onClick={async () => {
                setBusy("restore");
                try {
                  await api.backupRestore(pending.path);
                } catch (e) {
                  toast.error(errorMessage(e));
                  setBusy(null);
                }
              }}
            >
              Restore and restart
            </Button>
          </div>
        </div>
      )}
    </Panel>
  );
}

function UpdatesPanel() {
  const { data: version } = useQuery({ queryKey: ["app-version"], queryFn: getVersion });
  const [state, setState] = useState<{ kind: "idle" | "checking" | "current" | "error" } | { kind: "available"; update: Update }>({ kind: "idle" });
  const run = async () => {
    setState({ kind: "checking" });
    try {
      const update = await findUpdate();
      setState(update ? { kind: "available", update } : { kind: "current" });
    } catch (e) {
      setState({ kind: "error" });
      toast.error(`Couldn't check for updates: ${errorMessage(e)}`);
    }
  };
  return (
    <Panel className="flex items-center gap-3 px-4 py-3">
      <div className="flex-1">
        <p className="text-body font-medium">Waypoint {version ?? ""}</p>
        <p className="text-callout text-ink-3">
          {state.kind === "available"
            ? `Version ${state.update.version} is ready to install.`
            : state.kind === "current"
              ? "You have the latest version."
              : state.kind === "error"
                ? "The update server couldn't be reached."
                : "Updates are signed and checked before they install. Waypoint looks for one each time it starts."}
        </p>
      </div>
      {state.kind === "available" ? (
        <Button variant="primary" onClick={() => installUpdate(state.update)}>Install and restart</Button>
      ) : (
        <Button busy={state.kind === "checking"} onClick={run}>Check for updates</Button>
      )}
    </Panel>
  );
}

function FollowUpPanel() {
  const qc = useQueryClient();
  const { data } = useQuery({ queryKey: ["followups"], queryFn: api.followups });
  const [s, setS] = useState<FollowUpSettings | null>(null);
  useEffect(() => {
    if (data && !s) setS(data.settings);
  }, [data, s]);
  if (!s) return null;

  const save = async (next: FollowUpSettings, quiet = false) => {
    setS(next);
    try {
      await api.saveFollowupSettings(next);
      qc.invalidateQueries({ queryKey: ["followups"] });
      if (!quiet) toast.success("Follow-up timing saved");
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };

  const toggleNotify = async (on: boolean) => {
    if (on) {
      try {
        if (!(await isPermissionGranted()) && (await requestPermission()) !== "granted") {
          toast.error("Notifications are blocked. Allow Waypoint in your system's notification settings.");
          return;
        }
      } catch {
        // Some platforms don't report permission; sending still works.
      }
    }
    save({ ...s, notify: on }, true);
  };

  return (
    <Panel className="divide-y divide-hairline">
      <div className="flex items-center gap-3 px-4 py-3">
        <div className="flex-1">
          <p className="text-body font-medium">Notify me when a follow-up is due</p>
          <p className="text-callout text-ink-3">A system notification, at most once a day per job, while Waypoint is open.</p>
        </div>
        <Toggle checked={s.notify} onChange={toggleNotify} label="Follow-up notifications" />
      </div>
      <div className="flex flex-col gap-2.5 px-4 py-4">
        {CADENCE_FIELDS.map((f) => (
          <label key={f.key} className="flex items-center gap-3 text-callout">
            <span className="flex-1 text-ink-2">{f.label}</span>
            <Input
              type="number"
              min={1}
              max={f.key === "applied_max" ? 5 : 60}
              value={s.cadence[f.key]}
              onChange={(e) => setS({ ...s, cadence: { ...s.cadence, [f.key]: Number(e.target.value) || 1 } })}
              className="h-7 !w-16 shrink-0 text-right tabular"
            />
            <span className="w-20 shrink-0 text-ink-3">{f.unit}</span>
          </label>
        ))}
      </div>
      <div className="flex justify-end px-4 py-3">
        <Button size="sm" onClick={() => save(s)}>Save timing</Button>
      </div>
    </Panel>
  );
}

export function Settings() {
  const qc = useQueryClient();
  const { data: ai } = useQuery({ queryKey: ["ai-settings"], queryFn: api.getAiSettings });
  const [cfg, setCfg] = useState<LlmConfig>({ provider: "anthropic", model: "claude-opus-5-5", effort: "medium" });
  const [key, setKey] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (ai?.config) setCfg(ai.config);
  }, [ai?.config]);

  const provider = PROVIDERS.find((p) => p.id === cfg.provider)!;
  const hasKey = ai?.keys.find(([p]) => p === cfg.provider)?.[1] ?? false;
  const needsKey = cfg.provider !== "ollama";

  const save = async () => {
    setBusy(true);
    try {
      await api.saveAiSettings(cfg, key || undefined);
      setKey("");
      await qc.invalidateQueries({ queryKey: ["ai-settings"] });
      toast.success("AI settings saved");
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="drag flex h-[52px] shrink-0 items-center border-b border-hairline px-4" data-tauri-drag-region>
        <h1 className="text-headline font-semibold" data-tauri-drag-region>Settings</h1>
      </header>
      <div className="scroll flex-1">
        <div className="mx-auto flex max-w-[720px] flex-col gap-8 px-8 py-7">
          <section>
            <h2 className="text-headline font-semibold">AI model</h2>
            <p className="mt-0.5 mb-4 text-callout text-ink-2">Waypoint uses your own account, so you pay the provider directly. Waypoint has no servers of its own.</p>
            <div className="grid grid-cols-2 gap-2">
              {PROVIDERS.map((p) => (
                <button
                  key={p.id}
                  onClick={() => setCfg({ provider: p.id, model: p.models[0], effort: cfg.effort, base_url: null })}
                  className={clsx(
                    "flex flex-col items-start gap-1 rounded-[11px] p-3.5 text-left transition-all",
                    cfg.provider === p.id ? "bg-surface shadow-[0_0_0_2px_var(--color-selection)]" : "bg-surface shadow-[0_0_0_0.5px_var(--color-hairline)] hover:bg-raised",
                  )}
                >
                  <span className="flex w-full items-center justify-between text-body font-semibold">
                    {p.name}
                    {ai?.config?.provider === p.id && <Check className="size-4 text-teal" />}
                  </span>
                  <span className="text-callout text-ink-2">{p.blurb}</span>
                </button>
              ))}
            </div>
          </section>

          <Panel className="flex flex-col gap-4 p-5">
            <div className="grid grid-cols-2 gap-4">
              <Field label="Model" hint="Type any model name your provider supports">
                <Input list="model-options" value={cfg.model} onChange={(e) => setCfg({ ...cfg, model: e.target.value })} />
                <datalist id="model-options">
                  {provider.models.map((m) => (
                    <option key={m} value={m} />
                  ))}
                </datalist>
              </Field>
              {cfg.provider === "anthropic" ? (
                <Field label="Thoroughness" hint="Higher is more careful and uses more tokens">
                  <Segmented value={(cfg.effort ?? "medium") as "low" | "medium" | "high"} onChange={(v) => setCfg({ ...cfg, effort: v })} options={[{ value: "low", label: "Quick" }, { value: "medium", label: "Balanced" }, { value: "high", label: "Thorough" }]} />
                </Field>
              ) : (
                <Field label="Server address" hint={cfg.provider === "ollama" ? "Leave empty for http://localhost:11434" : "Leave empty for the default"}>
                  <Input value={cfg.base_url ?? ""} onChange={(e) => setCfg({ ...cfg, base_url: e.target.value })} placeholder={cfg.provider === "ollama" ? "http://localhost:11434/v1" : ""} />
                </Field>
              )}
            </div>
            {needsKey && (
              <Field
                label="API key"
                hint={
                  <span className="flex items-center gap-1.5">
                    <ShieldCheck className="size-3.5 text-teal" />
                    Stored in your system keychain, never in Waypoint's files.
                    {provider.keyUrl && (
                      <button className="text-selection hover:underline" onClick={() => openUrl(provider.keyUrl!)}>
                        Get a key
                      </button>
                    )}
                  </span>
                }
              >
                <div className="relative">
                  <KeyRound className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-ink-3" />
                  <Input type="password" value={key} onChange={(e) => setKey(e.target.value)} placeholder={hasKey ? "Saved. Paste a new key to replace it." : "Paste your key"} className="pl-7" />
                </div>
              </Field>
            )}
            <div className="flex items-center justify-end gap-2">
              {needsKey && hasKey && (
                <Button
                  variant="danger"
                  onClick={async () => {
                    await api.deleteApiKey(cfg.provider);
                    qc.invalidateQueries({ queryKey: ["ai-settings"] });
                    toast("Key removed from keychain");
                  }}
                >
                  Remove key
                </Button>
              )}
              <Button variant="primary" busy={busy} disabled={needsKey && !hasKey && !key} onClick={save}>
                Save
              </Button>
            </div>
          </Panel>

          <section>
            <h2 className="mb-3 text-headline font-semibold">Updates</h2>
            <UpdatesPanel />
          </section>

          <section>
            <h2 className="mb-3 text-headline font-semibold">Follow-up reminders</h2>
            <FollowUpPanel />
          </section>

          <section>
            <h2 className="mb-3 text-headline font-semibold">AI usage</h2>
            <Panel className="p-4">
              <UsagePanel />
            </Panel>
          </section>

          <section>
            <h2 className="text-headline font-semibold">Your data</h2>
            <p className="mt-1 mb-3 text-callout text-ink-2">Jobs, your profile and notes are stored in a local database on this computer. Waypoint has no account and no cloud. Job boards are contacted directly, and AI calls go straight to the provider you chose.</p>
            <BackupPanel />
          </section>
        </div>
      </div>
    </div>
  );
}
