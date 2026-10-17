import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { Toaster } from "sonner";
import App from "./App";
import "./styles.css";

const root = document.documentElement;
// Vibrancy is applied natively on macOS and Windows 11; Linux keeps a solid sidebar.
if (/Mac|Windows/.test(navigator.userAgent)) root.classList.add("vibrant");

const dark = window.matchMedia("(prefers-color-scheme: dark)");
const applyTheme = () => root.classList.toggle("dark", dark.matches);
applyTheme();
dark.addEventListener("change", applyTheme);

const queryClient = new QueryClient({
  defaultOptions: { queries: { staleTime: 15_000, refetchOnWindowFocus: false, retry: false } },
});

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
      <Toaster position="bottom-right" toastOptions={{ className: "!font-sans !text-callout" }} />
    </QueryClientProvider>
  </React.StrictMode>,
);
