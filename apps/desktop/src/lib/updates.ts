import { getVersion } from "@tauri-apps/api/app";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { toast } from "sonner";

export { getVersion };

/** Ask the update server whether a newer, signed build exists. `null` means up to date. */
export async function findUpdate(): Promise<Update | null> {
  return await check();
}

/** Download, verify the signature, install, then restart into the new version. */
export async function installUpdate(update: Update): Promise<void> {
  const id = toast.loading(`Downloading Waypoint ${update.version}…`);
  let total = 0;
  let got = 0;
  try {
    await update.downloadAndInstall((e) => {
      if (e.event === "Started") total = e.data.contentLength ?? 0;
      if (e.event === "Progress") {
        got += e.data.chunkLength;
        if (total) toast.loading(`Downloading Waypoint ${update.version}… ${Math.round((got / total) * 100)}%`, { id });
      }
      if (e.event === "Finished") toast.loading("Installing…", { id });
    });
    toast.success("Update installed. Restarting…", { id });
    await relaunch();
  } catch (e) {
    toast.error(`The update couldn't be installed: ${e instanceof Error ? e.message : String(e)}`, { id });
  }
}

const DISMISSED = "waypoint.update.dismissed";

/** Check once shortly after launch and offer the update; quiet when offline or up to date. */
export function checkOnLaunch() {
  setTimeout(async () => {
    try {
      const update = await findUpdate();
      if (!update) return;
      let dismissed: string | null = null;
      try {
        dismissed = localStorage.getItem(DISMISSED);
      } catch {
        // Storage can be unavailable; just offer it again.
      }
      if (dismissed === update.version) return;
      toast(`Waypoint ${update.version} is available`, {
        description: update.body?.split("\n")[0] || "Restart to get the latest fixes and features.",
        duration: Infinity,
        action: { label: "Install and restart", onClick: () => installUpdate(update) },
        cancel: {
          label: "Later",
          onClick: () => {
            try {
              localStorage.setItem(DISMISSED, update.version);
            } catch {
              // Nothing to remember it in; it will be offered next launch.
            }
          },
        },
      });
    } catch {
      // Offline, or no update feed yet. Settings has a manual check that reports errors.
    }
  }, 8000);
}
