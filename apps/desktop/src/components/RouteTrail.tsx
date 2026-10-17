import clsx from "clsx";
import { EXITS, ROUTE, STATUS_LABEL } from "../lib/format";
import type { Status } from "../lib/types";

/**
 * A job's position on its route: stops connected by a dotted path, the same motif as the
 * app icon. Clicking a stop moves the job there.
 */
export function RouteTrail({ status, onMove }: { status: Status; onMove?: (s: Status) => void }) {
  const exited = EXITS.includes(status);
  const reached = exited || status === "new" ? -1 : ROUTE.indexOf(status === "assessment" ? "responded" : status);

  return (
    <div className="flex items-center" role="list" aria-label="Application progress">
      {ROUTE.map((stop, i) => {
        const done = i <= reached;
        const current = i === reached;
        return (
          <div key={stop} className="flex items-center" role="listitem">
            {i > 0 && (
              <span
                aria-hidden
                className={clsx("mx-1 mb-[18px] h-0 w-5 border-t-2 border-dotted transition-colors", done ? "border-teal" : "border-fill-strong")}
              />
            )}
            <button
              disabled={!onMove}
              onClick={() => onMove?.(stop)}
              title={`Move to ${STATUS_LABEL[stop]}`}
              className="group flex flex-col items-center gap-1 disabled:cursor-default"
            >
              <span className="flex h-3 items-center">
                <span
                  className={clsx(
                    "block rounded-full transition-all duration-300 ease-spring",
                    current ? "size-3 bg-teal ring-4 ring-teal/20" : done ? "size-2 bg-teal" : "size-2 bg-fill-strong group-hover:bg-ink-3",
                  )}
                />
              </span>
              <span className={clsx("text-caption whitespace-nowrap", current ? "font-semibold text-ink" : "text-ink-3")}>{STATUS_LABEL[stop]}</span>
            </button>
          </div>
        );
      })}
      {exited && <span className="ml-3 text-caption font-medium text-coral">{STATUS_LABEL[status]}</span>}
    </div>
  );
}
