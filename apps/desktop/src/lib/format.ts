import type { Salary, Status } from "./types";

export const STATUS_LABEL: Record<Status, string> = {
  new: "New",
  saved: "Saved",
  evaluated: "Evaluated",
  applied: "Applied",
  responded: "Responded",
  assessment: "Assessment",
  interview: "Interview",
  offer: "Offer",
  hired: "Hired",
  rejected: "Rejected",
  discarded: "Discarded",
  skip: "Skipped",
};

/** The route a job travels; the Pipeline board draws these as stops. */
export const ROUTE: Status[] = ["saved", "evaluated", "applied", "responded", "interview", "offer", "hired"];

/** Where a job left the route, shown off to the side. */
export const EXITS: Status[] = ["rejected", "discarded", "skip"];

export function relativeTime(iso: string | null | undefined): string {
  if (!iso) return "";
  const then = new Date(iso).getTime();
  const days = Math.floor((Date.now() - then) / 86_400_000);
  if (days <= 0) return "Today";
  if (days === 1) return "Yesterday";
  if (days < 7) return `${days} days ago`;
  if (days < 30) return `${Math.floor(days / 7)} wk ago`;
  if (days < 365) return `${Math.floor(days / 30)} mo ago`;
  return `${Math.floor(days / 365)} yr ago`;
}

export function ageInDays(iso: string | null | undefined): number | null {
  if (!iso) return null;
  return Math.floor((Date.now() - new Date(iso).getTime()) / 86_400_000);
}

function compact(n: number, currency: string | null): string {
  const fmt = new Intl.NumberFormat(undefined, {
    style: currency ? "currency" : "decimal",
    currency: currency ?? undefined,
    notation: n >= 10_000 ? "compact" : "standard",
    maximumFractionDigits: n >= 10_000 ? 0 : 2,
  });
  try {
    return fmt.format(n);
  } catch {
    return `${n}`;
  }
}

export function formatSalary(s: Salary | null): string | null {
  if (!s || (s.min == null && s.max == null)) return null;
  const per = s.period === "hour" ? "/hr" : s.period === "month" ? "/mo" : s.period === "year" ? "/yr" : "";
  if (s.min != null && s.max != null && s.min !== s.max) return `${compact(s.min, s.currency)}–${compact(s.max, s.currency)}${per}`;
  return `${compact((s.min ?? s.max)!, s.currency)}${per}`;
}

export type Tone = "strong" | "good" | "moderate" | "weak" | "ineligible" | "none";

export function bandTone(band: string | null | undefined): Tone {
  switch (band) {
    case "Strong Fit":
      return "strong";
    case "Good Fit":
      return "good";
    case "Moderate Fit":
      return "moderate";
    case "Weak Fit":
    case "Poor Fit":
      return "weak";
    case "Ineligible":
      return "ineligible";
    default:
      return "none";
  }
}

export const SOURCE_LABEL: Record<string, string> = {
  greenhouse: "Greenhouse",
  lever: "Lever",
  lever_eu: "Lever",
  ashby: "Ashby",
  workday: "Workday",
  smartrecruiters: "SmartRecruiters",
  workable: "Workable",
  recruitee: "Recruitee",
  personio: "Personio",
  teamtailor: "Teamtailor",
  remotive: "Remotive",
  himalayas: "Himalayas",
  remoteok: "Remote OK",
  hackernews: "Hacker News",
  adzuna: "Adzuna",
  arbeitsagentur: "Arbeitsagentur",
  platsbanken: "Platsbanken",
  jobnet: "Jobnet",
  manual: "Added by you",
};

export function humanize(raw: string | null | undefined): string | null {
  if (!raw) return null;
  const spaced = raw.replace(/[_-]+/g, " ").replace(/([a-z])([A-Z])/g, "$1 $2").trim().toLowerCase();
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}
