import { useId } from "react";
import { motion } from "motion/react";
import clsx from "clsx";
import { bandTone, type Tone } from "../lib/format";

const TONE_COLOR: Record<Tone, string> = {
  strong: "var(--color-teal)",
  good: "var(--color-indigo)",
  moderate: "var(--color-amber)",
  weak: "var(--color-slate)",
  ineligible: "var(--color-coral)",
  none: "var(--color-fill-strong)",
};

/** Compact fit badge for list rows: the score in the band's color, or nothing. */
export function FitBadge({ score, band }: { score: number | null; band: string | null }) {
  const tone = bandTone(band);
  if (tone === "none") return null;
  return (
    <span className="tabular inline-flex h-5 min-w-8 items-center justify-center rounded-full px-1.5 text-caption font-semibold text-white" style={{ background: TONE_COLOR[tone] }} title={band ?? ""}>
      {score == null ? "—" : Math.round(score)}
    </span>
  );
}

/**
 * The fit dial: a 270° bearing arc, drawn in the icon's indigo→teal gradient up to the
 * score. Ineligible roles show a closed coral ring with no number.
 */
export function FitDial({ score, band, size = 104 }: { score: number | null; band: string | null; size?: number }) {
  const gradId = useId();
  const tone = bandTone(band);
  const stroke = 9;
  const r = (size - stroke) / 2;
  const sweep = 0.75; // 270°
  const circumference = 2 * Math.PI * r;
  const track = circumference * sweep;
  const value = score == null ? 0 : Math.max(0, Math.min(100, score)) / 100;

  return (
    <div className="relative shrink-0" style={{ width: size, height: size }}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} style={{ transform: "rotate(135deg)" }} aria-hidden>
        <defs>
          <linearGradient id={gradId} x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stopColor="var(--color-indigo)" />
            <stop offset="1" stopColor="var(--color-teal)" />
          </linearGradient>
        </defs>
        <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="var(--color-fill-strong)" strokeWidth={stroke} strokeLinecap="round" strokeDasharray={`${track} ${circumference}`} />
        {tone === "ineligible" ? (
          <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="var(--color-coral)" strokeWidth={stroke} strokeLinecap="round" strokeDasharray={`${track} ${circumference}`} />
        ) : (
          <motion.circle
            cx={size / 2}
            cy={size / 2}
            r={r}
            fill="none"
            stroke={tone === "weak" ? "var(--color-slate)" : `url(#${gradId})`}
            strokeWidth={stroke}
            strokeLinecap="round"
            initial={{ strokeDasharray: `0 ${circumference}` }}
            animate={{ strokeDasharray: `${track * value} ${circumference}` }}
            transition={{ type: "spring", stiffness: 120, damping: 22 }}
          />
        )}
      </svg>
      <div className="absolute inset-0 flex flex-col items-center justify-center">
        {tone === "ineligible" ? (
          <span className="text-callout font-semibold text-coral">Ineligible</span>
        ) : (
          <>
            <span className="tabular text-[1.75rem] leading-none font-semibold tracking-tight text-ink">{score == null ? "—" : Math.round(score)}</span>
            <span className={clsx("mt-1 text-caption font-medium", tone === "none" ? "text-ink-3" : "text-ink-2")}>{band ?? "Not scored"}</span>
          </>
        )}
      </div>
    </div>
  );
}
