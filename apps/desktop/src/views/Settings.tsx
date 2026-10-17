import { useQuery, useQueryClient } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import clsx from "clsx";
import { Check, KeyRound, ShieldCheck } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { Button, Field, Input, Panel, Segmented } from "../components/ui";
import { api, errorMessage } from "../lib/api";
import type { AiProvider, LlmConfig } from "../lib/types";

const PROVIDERS: { id: AiProvider; name: string; blurb: string; models: string[]; keyUrl?: string }[] = [
  { id: "anthropic", name: "Claude", blurb: "Anthropic's models. Strongest at careful, honest evaluation.", models: ["claude-opus-5-5", "claude-sonnet-5-5", "claude-haiku-5-5"], keyUrl: "https://console.anthropic.com/settings/keys" },
  { id: "openai", name: "OpenAI", blurb: "GPT models through your OpenAI account.", models: ["gpt-5", "gpt-5-mini"], keyUrl: "https://platform.openai.com/api-keys" },
  { id: "openrouter", name: "OpenRouter", blurb: "One key for many models, including free ones.", models: ["anthropic/claude-opus-5-5", "anthropic/claude-sonnet-5-5", "openai/gpt-5"], keyUrl: "https://openrouter.ai/keys" },
  { id: "ollama", name: "Ollama", blurb: "Free and fully private: models run on this computer.", models: ["llama3.3", "qwen3", "gemma3"] },
];

function UsagePanel() {
  const { data: usage = [] } = useQuery({ queryKey: ["usage"], queryFn: api.tokenUsage });
  if (!usage.length) return <p className="text-callout text-ink-3">No AI calls yet. Token use appears here after your first scored job.</p>;
  return (
    <table className="w-full text-callout">
      <thead>
        <tr className="text-left text-caption text-ink-3">
          <th className="pb-2 font-medium">Model</th>
          <th className="pb-2 text-right font-medium">Jobs scored</th>
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
            <h2 className="mb-3 text-headline font-semibold">AI usage</h2>
            <Panel className="p-4">
              <UsagePanel />
            </Panel>
          </section>

          <section>
            <h2 className="text-headline font-semibold">Your data</h2>
            <p className="mt-1 text-callout text-ink-2">Jobs, your profile and notes are stored in a local database on this computer. Waypoint has no account and no cloud. Job boards are contacted directly, and AI calls go straight to the provider you chose.</p>
          </section>
        </div>
      </div>
    </div>
  );
}
