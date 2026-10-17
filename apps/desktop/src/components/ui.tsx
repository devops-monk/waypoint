import { forwardRef, type ButtonHTMLAttributes, type InputHTMLAttributes, type ReactNode, type TextareaHTMLAttributes } from "react";
import clsx from "clsx";
import { Loader2 } from "lucide-react";

type Variant = "primary" | "secondary" | "plain" | "danger";

export const Button = forwardRef<HTMLButtonElement, ButtonHTMLAttributes<HTMLButtonElement> & { variant?: Variant; busy?: boolean; icon?: ReactNode; size?: "sm" | "md" }>(
  function Button({ variant = "secondary", busy, icon, size = "md", className, children, disabled, ...rest }, ref) {
    return (
      <button
        ref={ref}
        disabled={disabled || busy}
        className={clsx(
          "inline-flex select-none items-center justify-center gap-1.5 rounded-control font-medium whitespace-nowrap transition-[background,transform,opacity] duration-150 active:scale-[0.97] disabled:pointer-events-none disabled:opacity-45",
          size === "md" ? "h-8 px-3.5 text-callout" : "h-6.5 px-2.5 text-caption",
          variant === "primary" && "bg-selection text-white shadow-[0_1px_0_rgb(255_255_255/0.2)_inset,0_1px_2px_rgb(0_0_0/0.15)] hover:brightness-110",
          variant === "secondary" && "bg-surface text-ink shadow-[0_0_0_0.5px_var(--color-hairline),0_1px_1.5px_rgb(0_0_0/0.08)] hover:bg-raised",
          variant === "plain" && "text-ink-2 hover:bg-fill hover:text-ink",
          variant === "danger" && "text-coral hover:bg-coral/10",
          className,
        )}
        {...rest}
      >
        {busy ? <Loader2 className="size-3.5 animate-spin" /> : icon}
        {children}
      </button>
    );
  },
);

export function IconButton({ label, className, children, ...rest }: ButtonHTMLAttributes<HTMLButtonElement> & { label: string }) {
  return (
    <button
      aria-label={label}
      title={label}
      className={clsx("inline-flex size-7 items-center justify-center rounded-control text-ink-2 transition-colors hover:bg-fill hover:text-ink active:bg-fill-strong", className)}
      {...rest}
    >
      {children}
    </button>
  );
}

const fieldBase =
  "w-full rounded-control bg-surface px-2.5 text-body text-ink placeholder:text-ink-3 shadow-[0_0_0_0.5px_var(--color-hairline),0_0.5px_1px_rgb(0_0_0/0.06)_inset] outline-none transition-shadow focus:shadow-[0_0_0_3px_color-mix(in_srgb,var(--color-selection)_35%,transparent)]";

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(function Input({ className, ...rest }, ref) {
  return <input ref={ref} className={clsx(fieldBase, "h-8", className)} {...rest} />;
});

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaHTMLAttributes<HTMLTextAreaElement>>(function Textarea({ className, ...rest }, ref) {
  return <textarea ref={ref} className={clsx(fieldBase, "min-h-24 resize-y py-2 leading-relaxed", className)} {...rest} />;
});

export function Field({ label, hint, children, className }: { label: string; hint?: ReactNode; children: ReactNode; className?: string }) {
  return (
    <label className={clsx("flex flex-col gap-1.5", className)}>
      <span className="text-callout font-medium text-ink-2">{label}</span>
      {children}
      {hint && <span className="text-caption text-ink-3">{hint}</span>}
    </label>
  );
}

export function Toggle({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      aria-label={label}
      onClick={() => onChange(!checked)}
      className={clsx("relative h-[22px] w-[38px] shrink-0 rounded-full transition-colors duration-200", checked ? "bg-teal" : "bg-fill-strong")}
    >
      <span
        className={clsx(
          "absolute top-[2px] left-[2px] size-[18px] rounded-full bg-white shadow-[0_1px_3px_rgb(0_0_0/0.25)] transition-transform duration-300 ease-spring",
          checked && "translate-x-4",
        )}
      />
    </button>
  );
}

export function Segmented<T extends string>({ value, options, onChange }: { value: T; options: { value: T; label: string }[]; onChange: (v: T) => void }) {
  return (
    <div className="inline-flex rounded-[8px] bg-fill p-0.5" role="tablist">
      {options.map((o) => (
        <button
          key={o.value}
          role="tab"
          aria-selected={o.value === value}
          onClick={() => onChange(o.value)}
          className={clsx(
            "h-6 rounded-[6px] px-3 text-callout font-medium transition-all duration-200",
            o.value === value ? "bg-surface text-ink shadow-[0_0_0_0.5px_var(--color-hairline),0_1px_2px_rgb(0_0_0/0.1)]" : "text-ink-2 hover:text-ink",
          )}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function Kbd({ children }: { children: ReactNode }) {
  return <kbd className="inline-flex h-5 min-w-5 items-center justify-center rounded-[5px] bg-fill px-1 font-sans text-caption text-ink-3">{children}</kbd>;
}

export function Tag({ children, tone = "neutral" }: { children: ReactNode; tone?: "neutral" | "teal" | "indigo" | "amber" | "coral" }) {
  return (
    <span
      className={clsx(
        "inline-flex h-5 items-center rounded-full px-2 text-caption font-medium whitespace-nowrap",
        tone === "neutral" && "bg-fill text-ink-2",
        tone === "teal" && "bg-teal/12 text-teal",
        tone === "indigo" && "bg-indigo/12 text-indigo",
        tone === "amber" && "bg-amber/15 text-amber",
        tone === "coral" && "bg-coral/12 text-coral",
      )}
    >
      {children}
    </span>
  );
}

export function SectionTitle({ children, action }: { children: ReactNode; action?: ReactNode }) {
  return (
    <div className="mb-2 flex items-center justify-between">
      <h3 className="text-callout font-semibold text-ink">{children}</h3>
      {action}
    </div>
  );
}

export function Panel({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={clsx("rounded-panel bg-surface shadow-[0_0_0_0.5px_var(--color-hairline),0_1px_2px_rgb(0_0_0/0.04)]", className)}>{children}</div>;
}
