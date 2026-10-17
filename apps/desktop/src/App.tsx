import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { Sidebar, type View } from "./components/Sidebar";
import { CommandPalette } from "./components/CommandPalette";
import { AddJobSheet } from "./components/AddJobSheet";
import { Discover } from "./views/Discover";
import { Pipeline } from "./views/Pipeline";
import { Sources } from "./views/Sources";
import { ProfileView } from "./views/Profile";
import { Settings } from "./views/Settings";

export default function App() {
  const [view, setView] = useState<View>("discover");
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [addOpen, setAddOpen] = useState(false);
  const [focusJob, setFocusJob] = useState<number | null>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPaletteOpen((o) => !o);
      } else if (mod && e.key.toLowerCase() === "n") {
        e.preventDefault();
        setAddOpen(true);
      } else if (mod && e.key === ",") {
        e.preventDefault();
        setView("settings");
      } else if (mod && /^[1-5]$/.test(e.key)) {
        e.preventDefault();
        setView((["discover", "pipeline", "sources", "profile", "settings"] as View[])[Number(e.key) - 1]);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const openJob = (id: number) => {
    setFocusJob(id);
    setView("discover");
  };

  return (
    <div className="flex h-full">
      <Sidebar view={view} onNavigate={setView} onAdd={() => setAddOpen(true)} onSearch={() => setPaletteOpen(true)} />
      <main className="relative flex min-w-0 flex-1 flex-col bg-window shadow-[-0.5px_0_0_var(--color-hairline)]">
        <AnimatePresence mode="wait" initial={false}>
          <motion.div
            key={view}
            className="flex min-h-0 flex-1 flex-col"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.12 }}
          >
            {view === "discover" && <Discover focusJob={focusJob} onFocusHandled={() => setFocusJob(null)} onGoTo={setView} />}
            {view === "pipeline" && <Pipeline onOpenJob={openJob} />}
            {view === "sources" && <Sources />}
            {view === "profile" && <ProfileView />}
            {view === "settings" && <Settings />}
          </motion.div>
        </AnimatePresence>
      </main>
      <CommandPalette open={paletteOpen} onClose={() => setPaletteOpen(false)} onNavigate={setView} onOpenJob={openJob} onAdd={() => setAddOpen(true)} />
      <AddJobSheet open={addOpen} onClose={() => setAddOpen(false)} onAdded={openJob} />
    </div>
  );
}
