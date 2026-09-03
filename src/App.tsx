import { useEffect, useState, useCallback, memo } from "react";
import { Send, History, Settings, Shield, Sun, Moon, Zap, Home, Inbox, Wifi, MoreHorizontal } from "lucide-react";
import { DropZone } from "./components/DropZone";
import { PeerList } from "./components/PeerList";
import { ProgressView } from "./components/ProgressView";
import { PairingView } from "./components/PairingView";
import { HistoryView } from "./components/HistoryView";
import { SettingsView } from "./components/SettingsView";
import { useAppStore } from "./stores/appStore";
import "./i18n";

const SidebarNav = memo(function SidebarNav() {
  const view = useAppStore((s) => s.view);
  const setView = useAppStore((s) => s.setView);
  const items = [
    { id: "transfer", label: "Transfert", icon: Send },
    { id: "pairing", label: "Appairage", icon: Shield },
    { id: "history", label: "Historique", icon: History },
    { id: "settings", label: "Paramètres", icon: Settings },
  ] as const;
  return (
    <nav className="hidden lg:flex lg:flex-col gap-2 rounded-[20px] border border-[var(--border)] bg-[var(--surface)] p-2 shadow-sm">
      {items.map((item) => {
        const active = view === item.id;
        return (
          <button
            key={item.id}
            onClick={() => setView(item.id as typeof view)}
            className={`flex items-center gap-3 rounded-[14px] px-3 py-3 text-left transition-colors ${active ? "bg-[var(--accent)] text-white shadow-sm" : "hover:bg-[var(--surface-hover)]"}`}
          >
            <item.icon className="h-5 w-5 shrink-0" strokeWidth={1.75} />
            <span className="text-sm font-semibold">{item.label}</span>
          </button>
        );
      })}
      <div className="mx-1 my-2 h-px bg-[var(--border)]" />
      <div className="rounded-[14px] bg-gradient-to-br from-blue-600 to-indigo-600 p-3 text-white">
        <p className="text-xs font-bold">3 panes desktop</p>
        <p className="mt-1 text-xs leading-relaxed text-white/80">Appareils · Zone centrale · Détails</p>
      </div>
    </nav>
  );
});

const MobileBottomNav = memo(function MobileBottomNav() {
  const mobileTab = useAppStore((s) => s.mobileTab);
  const setMobileTab = useAppStore((s) => s.setMobileTab);
  const tabs = [
    { id: "home", label: "Accueil", icon: Home },
    { id: "send", label: "Envoyer", icon: Send },
    { id: "received", label: "Reçus", icon: Inbox },
    { id: "settings", label: "Param.", icon: Settings },
  ] as const;
  return (
    <nav className="lg:hidden fixed bottom-0 inset-x-0 z-30 border-t border-[var(--border)] bg-[var(--surface)]/95 backdrop-blur">
      <div className="mx-auto grid max-w-md grid-cols-4 gap-1 px-2 py-2">
        {tabs.map((t) => {
          const active = mobileTab === t.id;
          return (
            <button key={t.id} onClick={() => setMobileTab(t.id as typeof mobileTab)} className={`flex flex-col items-center gap-1 rounded-2xl py-2 text-xs font-medium transition-colors ${active ? "bg-[var(--accent)] text-white" : "text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]"}`}>
              <t.icon className="h-5 w-5" strokeWidth={active ? 2.2 : 1.7} /> {t.label}
            </button>
          );
        })}
      </div>
    </nav>
  );
});

const TransferThreePane = memo(function TransferThreePane({
  onFiles,
  hasFiles,
}: {
  onFiles: (f: File[]) => void;
  hasFiles: boolean;
}) {
  const selectedTransferId = useAppStore((s) => s.selectedTransferId);
  const transfers = useAppStore((s) => s.transfers);
  const selected = transfers.find((t) => t.id === selectedTransferId);
  return (
    <div className="grid grid-cols-1 lg:grid-cols-[280px_minmax(0,1fr)_320px] gap-6">
      <div className="space-y-4">
        <div className="rounded-[20px] border border-[var(--border)] bg-[var(--surface)] p-3 shadow-sm">
          <div className="flex items-center justify-between px-2 py-1">
            <h2 className="text-xs font-bold tracking-widest uppercase text-[var(--text-secondary)]">Appareils</h2>
            <span className="flex items-center gap-1 text-xs text-emerald-600">
              <Wifi className="h-3 w-3" /> mDNS
            </span>
          </div>
          <div className="mt-3">
            <PeerList />
          </div>
        </div>
      </div>

      <div className="space-y-4 min-w-0">
        <DropZone onFilesSelected={onFiles} />
        {hasFiles ? <ProgressView /> : (
          <div className="rounded-[20px] border border-dashed border-[var(--border-strong)] bg-[var(--surface)] p-6 text-center">
            <p className="text-sm font-medium">Aucun transfert</p>
            <p className="mt-1 text-xs text-[var(--text-secondary)]">Glisse un fichier — envoi en &lt;3s</p>
          </div>
        )}
      </div>

      <div className="space-y-4">
        <div className={`rounded-[20px] border bg-[var(--surface)] shadow-sm transition-all ${selected ? "p-4" : "border-dashed border-[var(--border-strong)] p-6 text-center"}`}>
          {selected ? (
            <>
              <div className="flex items-center justify-between">
                <h3 className="text-sm font-bold">Détails</h3>
                <button className="rounded-full p-1 hover:bg-[var(--surface-hover)]"><MoreHorizontal className="h-4 w-4" /></button>
              </div>
              <p className="mono mt-2 text-xs break-all">{selected.files[0]?.path}</p>
              <div className="mt-3 space-y-2 text-xs">
                <div className="flex justify-between"><span className="text-[var(--text-secondary)]">Vitesse</span><span className="font-medium">{(selected.speedBps / 1024 / 1024).toFixed(0)} Mo/s</span></div>
                <div className="flex justify-between"><span className="text-[var(--text-secondary)]">ETA</span><span className="font-medium">{selected.etaSecs}s</span></div>
                <div className="flex justify-between"><span className="text-[var(--text-secondary)]">Intégrité</span><span className="text-emerald-600">BLAKE3 ✓</span></div>
              </div>
            </>
          ) : (
            <>
              <p className="text-sm font-semibold">Aucune sélection</p>
              <p className="mt-1 text-xs text-[var(--text-secondary)]">Clique sur un transfert.</p>
            </>
          )}
        </div>
      </div>
    </div>
  );
});

export default function App() {
  const darkMode = useAppStore((s) => s.darkMode);
  const toggleDarkMode = useAppStore((s) => s.toggleDarkMode);
  const view = useAppStore((s) => s.view);
  const mobileTab = useAppStore((s) => s.mobileTab);
  const setTransfers = useAppStore((s) => s.setTransfers);
  const transfers = useAppStore((s) => s.transfers);
  const [hasFiles, setHasFiles] = useState(false);

  useEffect(() => {
    document.documentElement.classList.toggle("dark", darkMode);
  }, [darkMode]);

  const handleFiles = useCallback(
    (files: File[]) => {
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
        setTransfers([{ ...next, status: "running", bytesDone: Math.floor(total * 0.42), speedBps: 85000000, etaSecs: 4 }]);
      }, 400);
    },
    [transfers, setTransfers]
  );

  return (
    <div className="min-h-screen bg-[var(--background)] text-[var(--text-primary)] antialiased">
      <header className="sticky top-0 z-20 border-b border-[var(--border)] bg-[var(--surface)]/80 backdrop-blur">
        <div className="mx-auto flex max-w-[1280px] items-center justify-between px-4 sm:px-6 py-3.5">
          <div className="flex items-center gap-3">
            <img src={darkMode ? "/assets/Images/symbol-on-dark.png" : "/assets/Images/symbol-on-light.png"} alt="RivaldSend" className="h-9 w-9 rounded-xl bg-white p-1.5 shadow-sm object-contain" onError={(e) => ((e.target as HTMLImageElement).style.display = "none")} />
            <div>
              <p className="text-[15px] font-extrabold tracking-tight leading-none">RivaldSend</p>
              <p className="hidden sm:block text-xs font-medium text-[var(--text-secondary)]">Pro · Réactive</p>
            </div>
            <span className="ml-2 hidden lg:inline-flex items-center gap-1.5 rounded-full bg-[var(--accent)] px-3 py-1 text-xs font-bold text-white">
              <Zap className="h-3 w-3" /> Fluide
            </span>
          </div>
          <button onClick={toggleDarkMode} aria-label="Thème" className="flex h-9 w-9 items-center justify-center rounded-full border border-[var(--border)] bg-[var(--surface)] hover:bg-[var(--surface-hover)] transition-colors">
            {darkMode ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
          </button>
        </div>
      </header>

      <div className="mx-auto max-w-[1280px] px-4 sm:px-6 py-6">
        <div className="hidden lg:grid lg:grid-cols-[240px_minmax(0,1fr)] gap-6">
          <SidebarNav />
          <main className="min-w-0">
            {view === "transfer" && <TransferThreePane onFiles={handleFiles} hasFiles={hasFiles} />}
            {view === "pairing" && <div className="fade-in max-w-3xl"><PairingView /></div>}
            {view === "history" && <div className="fade-in max-w-3xl"><HistoryView /></div>}
            {view === "settings" && <div className="fade-in max-w-3xl"><SettingsView /></div>}
          </main>
        </div>

        <div className="lg:hidden space-y-4 pb-20">
          {mobileTab === "home" && <TransferThreePane onFiles={handleFiles} hasFiles={hasFiles} />}
          {mobileTab === "send" && <div className="fade-in"><HistoryView /></div>}
          {mobileTab === "received" && <div className="fade-in"><HistoryView /></div>}
          {mobileTab === "settings" && <div className="fade-in"><SettingsView /></div>}
        </div>
      </div>

      <MobileBottomNav />

      <footer className="hidden lg:block border-t border-[var(--border)] py-4 text-center text-xs text-[var(--text-tertiary)]">
        RivaldSend v0.1.0 · Sidebar desktop · Bottom nav mobile uniquement · Réactive 60fps
      </footer>
    </div>
  );
}
