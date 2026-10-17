import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { JobPicker } from "../components/docs";
import { Segmented } from "../components/ui";
import { api } from "../lib/api";
import { CoverLetterStudio } from "./CoverLetterStudio";
import { ResumeStudio } from "./ResumeStudio";

export type DocKind = "resume" | "letter";

/** Resume and cover letter for one job, side by side in one place. */
export function Documents({ target }: { target: { jobId: number | null; kind: DocKind } }) {
  const [jobId, setJobId] = useState<number | null>(target.jobId);
  const [kind, setKind] = useState<DocKind>(target.kind);

  useEffect(() => {
    setJobId(target.jobId);
    setKind(target.kind);
  }, [target]);

  const { data: jobs = [] } = useQuery({
    queryKey: ["jobs", "document-picker"],
    queryFn: () => api.listJobs({ statuses: ["saved", "evaluated", "applied", "responded", "assessment", "interview", "offer"], limit: 300 }),
  });
  const job = jobs.find((j) => j.id === jobId);

  const leading = (
    <>
      <Segmented<DocKind> value={kind} onChange={setKind} options={[{ value: "resume", label: "Resume" }, { value: "letter", label: "Cover letter" }]} />
      <JobPicker jobs={jobs} jobId={jobId} onChange={setJobId} baseLabel={kind === "resume" ? "Base resume" : undefined} />
    </>
  );

  return kind === "resume" ? <ResumeStudio jobId={jobId} job={job} leading={leading} /> : <CoverLetterStudio jobId={jobId} job={job} leading={leading} />;
}
