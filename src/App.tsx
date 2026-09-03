import { useEffect, useState } from "react";
import { Send, History, Settings, Shield, Sun, Moon, Zap } from "lucide-react";
import { DropZone } from "./components/DropZone";
import { PeerList } from "./components/PeerList";
import { ProgressView } from "./components/ProgressView";
import { PairingView } from "./components/PairingView";
import { HistoryView } from "./components/HistoryView";
import { SettingsView } from "./components/SettingsView";
import { useAppStore } from "./stores/appStore";
import "./i18n";

export default function App() {
  const { view, setView, darkMode, toggleDarkMode, setTransfers, transfers } = useAppStore();
  const [hasFiles, setHasFiles] = useState(false);

  useEffect(() => {
    document.documentElement.classList.toggle("dark", darkMode);
  }, [darkMode]);

  const handleFiles = (files: File[]) => {
    setHasFiles(files.length > 0);
    const total = files.reduce((s, f) => s + f.size, 0);
    const next = {
      id: crypto.randomUUID(),
      files: files.map((f) => ({ path: f.name, size: f.size, blake3: "0".repeat(64) })),
      totalBytes: total,
      bytesDone: 0,
      speedBps: 0,
      etaSecs: 0,
      status: "queued" as const,
      peerId: "1",
      createdAt: new Date().toISOString(),
    };
    setTransfers([...transfers, next]);
    setTimeout(() => {
      setTransfers([{ ...next, status: "running", bytesDone: total * 0.42, speedBps: 85000000, etaSecs: 4 }]);
    }, 600);
  };

  return (
    <div className="min-h-screen bg-[var(--background)] text-[var(--text-primary)]">
      <header className="sticky top-0 z-10 border-b border-[var(--border)] bg-[var(--surface)]/80 backdrop-blur">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-3">
          <div className="flex items-center gap-3">
            <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-[var(--accent)] text-white font-bold text-sm">RS</div>
            <div>
              <p className="text-sm font-semibold leading-none">RivaldSend</p>
              <p className="text-xs text-[var(--text-secondary)]">Local · Rapide · Fiable</p>
            </div>
            <span className="ml-2 hidden items-center gap-1 rounded-full bg-[var(--accent-light)] px-2.5 py-1 text-xs font-medium text-[var(--accent)] sm:inline-flex">
              <Zap className="h-3 w-3" /> P0 Prototype
            </span>
          </div>
          <div className="flex items-center gap-2">
            <button onClick={toggleDarkMode} className="rounded-full p-2 hover:bg-[var(--surface-hover)]">
              {darkMode ? <Sun className="h-5 w-5" /> : <Moon className="h-5 w-5" />}
            </button>
            <div className="hidden sm:flex items-center gap-1 rounded-full border border-[var(--border)] px-3 py-1 text-xs">
              <Shield className="h-3.5 w-3.5 text-[var(--success)]" /> TLS 1.3 · BLAKE3
            </div>
          </div>
        </div>
      </header>

      <div className="mx-auto grid max-w-6xl grid-cols-12 gap-6 px-6 py-6">
        <aside className="col-span-12 lg:col-span-3">
          <nav className="sticky top-[65px] space-y-2">
            {[
              { id: "transfer", label: "Transfert", icon: Send },
              { id: "pairing", label: "Appairage", icon: Shield },
              { id: "history", label: "Historique", icon: History },
              { id: "settings", label: "Paramètres", icon: Settings },
            ].map((item) => (
              <button
                key={item.id}
                onClick={() => setView(item.id as typeof view)}
                className={`flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-sm font-medium transition-colors ${view === item.id ? "bg-[var(--accent)] text-white" : "hover:bg-[var(--surface)] border border-transparent hover:border-[var(--border)]"}`}
              >
                <item.icon className="h-4 w-4" /> {item.label}
              </button>
            ))}
            <div className="pt-4">
              <div className="rounded-2xl bg-[var(--surface)] border border-[var(--border)] p-4">
                <p className="text-xs font-semibold">Astuce</p>
                <p className="mt-1 text-xs text-[var(--text-secondary)] leading-relaxed">Le pairing utilise TLS-PSK (HKDF) — MITM impossible sans le code à 6 chiffres. Voir §5.2 de la spec.</p>
              </div>
            </div>
          </nav>
        </aside>

        <main className="col-span-12 lg:col-span-9 space-y-6">
          {view === "transfer" && (
            <>
              <DropZone onFilesSelected={handleFiles} />
              {hasFiles && <ProgressView />}
              <PeerList />
              <div className="rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-4 text-xs text-[var(--text-secondary)]">
                Protocole: HTTP/1.1 + TLS 1.3 · Endpoints <span className="mono">/v1/negotiate · /v1/transfers · /v1/transfers/{"{id}"}/chunks</span> · Chunking par paliers 16 Mo → 4 Mo
              </div>
            </>
          )}
          {view === "pairing" && <PairingView />}
          {view === "history" && <HistoryView />}
          {view === "settings" && <SettingsView />}
        </main>
      </div>

      <footer className="border-t border-[var(--border)] py-4 text-center text-xs text-[var(--text-secondary)]">RivaldSend v0.1.0 — P0 Prototype · HTTP/axum · BLAKE3 · Workspace 3 crates · Tauri IPC strict (zéro byte de fichier)</footer>
    </div>
  );
}
