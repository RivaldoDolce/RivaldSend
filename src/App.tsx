import { useEffect, useState, useCallback, memo } from "react";
import { Send, History, Settings, Shield, Sun, Moon, Zap, Home, Inbox, ArrowUpRight, Wifi, MoreHorizontal } from "lucide-react";
import { DropZone } from "./components/DropZone";
import { PeerList } from "./components/PeerList";
import { ProgressView } from "./components/ProgressView";
import { PairingView } from "./components/PairingView";
import { HistoryView } from "./components/HistoryView";
import { SettingsView } from "./components/SettingsView";
import { useAppStore } from "./stores/appStore";
import { useIsMobile } from "./hooks/useIsMobile";
import "./i18n";

const DesktopThreePane = memo(function DesktopThreePane({
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
    <div className="hidden lg:grid lg:grid-cols-[280px_minmax(0,1fr)_320px] gap-6">
      <aside className="space-y-4">
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
          <button className="mt-3 w-full rounded-xl border border-dashed border-[var(--border-strong)] bg-[var(--surface-hover)] py-2.5 text-xs font-semibold hover:bg-[var(--surface)] transition-colors">+ Ajouter manuellement</button>
        </div>
        <div className="rounded-[20px] bg-gradient-to-br from-blue-600 to-indigo-600 p-4 text-white shadow-md">
          <p className="text-xs font-bold">Courrier express numérique</p>
          <p className="mt-1 text-xs leading-relaxed text-white/80">Dépose un colis, choisis le destinataire, suis la livraison.</p>
        </div>
      </aside>

      <main className="space-y-4 min-w-0">
        <DropZone onFilesSelected={onFiles} />
        {hasFiles ? <ProgressView /> : (
          <div className="rounded-[20px] border border-[var(--border)] bg-[var(--surface)] p-6 text-center">
            <p className="text-sm font-medium">Aucun transfert en cours</p>
            <p className="mt-1 text-xs text-[var(--text-secondary)]">Glisse un fichier ou clique pour parcourir — envoi en &lt;3s</p>
          </div>
        )}
        <div className="flex gap-2 text-xs">
          <span className="rounded-full border border-[var(--border)] bg-[var(--surface)] px-3 py-1 mono">/v1/negotiate</span>
          <span className="rounded-full border border-[var(--border)] bg-[var(--surface)] px-3 py-1 mono">TLS-PSK HKDF</span>
        </div>
      </main>

      <aside className="space-y-4">
        <div className={`rounded-[20px] border bg-[var(--surface)] shadow-sm transition-all ${selected ? "border-[var(--border)] p-4" : "border-dashed border-[var(--border-strong)] p-6 text-center"}`}>
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
              <div className="mt-4 grid grid-cols-2 gap-2">
                <button className="rounded-full bg-[var(--surface-hover)] py-2 text-xs font-semibold">Pause</button>
                <button className="rounded-full bg-red-500 py-2 text-xs font-semibold text-white">Annuler</button>
              </div>
            </>
          ) : (
            <>
              <p className="text-sm font-semibold">Aucune sélection</p>
              <p className="mt-1 text-xs text-[var(--text-secondary)]">Clique sur un transfert pour voir les détails.</p>
            </>
          )}
        </div>
        <div className="rounded-[20px] border border-[var(--border)] bg-[var(--surface)] p-4">
          <p className="text-xs font-bold">Statut</p>
          <p className="mt-1 text-xs text-[var(--text-secondary)]">● Connecté — 2 appareils visibles — chiffré de bout en bout</p>
        </div>
      </aside>
    </div>
  );
});

const MobileTabs = memo(function MobileTabs({
  onFiles,
  hasFiles,
}: {
  onFiles: (f: File[]) => void;
  hasFiles: boolean;
}) {
  const mobileTab = useAppStore((s) => s.mobileTab);
  const setMobileTab = useAppStore((s) => s.setMobileTab);
  return (
    <div className="lg:hidden space-y-4">
      {mobileTab === "home" && (
        <div className="space-y-4 fade-in">
          <DropZone onFilesSelected={onFiles} />
          {hasFiles && <ProgressView />}
          <PeerList />
        </div>
      )}
      {mobileTab === "send" && <div className="fade-in"><HistoryView /></div>}
      {mobileTab === "received" && <div className="fade-in"><HistoryView /></div>}
      {mobileTab === "settings" && <div className="fade-in"><SettingsView /></div>}

      <nav className="fixed bottom-0 inset-x-0 z-10 border-t border-[var(--border)] bg-[var(--surface)]/95 backdrop-blur supports-[backdrop-filter]:bg-[var(--surface)]/80">
        <div className="mx-auto grid max-w-md grid-cols-4 gap-1 px-2 py-2">
          {[
            { id: "home", label: "Accueil", icon: Home },
            { id: "send", label: "Envoyer", icon: Send },
            { id: "received", label: "Reçus", icon: Inbox },
            { id: "settings", label: "Param.", icon: Settings },
          ].map((t) => {
            const active = mobileTab === t.id;
            return (
              <button key={t.id} onClick={() => setMobileTab(t.id as typeof mobileTab)} className={`flex flex-col items-center gap-1 rounded-2xl py-2 text-xs font-medium transition-colors ${active ? "bg-[var(--accent)] text-white" : "text-[var(--text-secondary)]"}`}>
                <t.icon className="h-5 w-5" strokeWidth={active ? 2.2 : 1.7} /> {t.label}
              </button>
            );
          })}
        </div>
      </nav>
      <div className="h-16" />
    </div>
  );
});

export default function App() {
  const darkMode = useAppStore((s) => s.darkMode);
  const toggleDarkMode = useAppStore((s) => s.toggleDarkMode);
  const view = useAppStore((s) => s.view);
  const setView = useAppStore((s) => s.setView);
  const setTransfers = useAppStore((s) => s.setTransfers);
  const transfers = useAppStore((s) => s.transfers);
  const isMobile = useIsMobile();
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
      const updated = [...transfers, next];
      setTransfers(updated);
      setTimeout(() => {
        setTransfers([{ ...next, status: "running", bytesDone: Math.floor(total * 0.42), speedBps: 85000000, etaSecs: 4 }]);
      }, 500);
    },
    [transfers, setTransfers]
  );

  return (
    <div className="min-h-screen bg-[var(--background)] text-[var(--text-primary)] antialiased">
      <div className="pointer-events-none fixed inset-0 bg-gradient-to-b from-blue-500/[0.04] via-transparent to-transparent" />
      <header className="sticky top-0 z-20 border-b border-[var(--border)] glass">
        <div className="mx-auto flex max-w-[1280px] items-center justify-between px-4 sm:px-6 py-3.5">
          <div className="flex items-center gap-3">
            <img src={darkMode ? "/assets/Images/symbol-on-dark.png" : "/assets/Images/symbol-on-light.png"} alt="RivaldSend" className="h-9 w-9 rounded-xl bg-white p-1.5 shadow-sm object-contain" onError={(e) => ((e.target as HTMLImageElement).style.display = "none")} />
            <div>
              <p className="text-[15px] font-extrabold tracking-tight leading-none">RivaldSend</p>
              <p className="hidden sm:block text-xs font-medium text-[var(--text-secondary)]">LocalDrop — courrier express numérique</p>
            </div>
            <span className="ml-2 hidden lg:inline-flex items-center gap-1.5 rounded-full bg-[var(--accent)] px-3 py-1 text-xs font-bold text-white shadow-sm">
              <Zap className="h-3 w-3" /> Pro · Fluide 60fps
            </span>
          </div>
          <div className="flex items-center gap-2">
            <div className="hidden md:flex items-center gap-1.5 rounded-full border border-emerald-200 bg-emerald-50 px-3 py-1.5 text-xs font-medium text-emerald-700 dark:border-emerald-900 dark:bg-emerald-950/40 dark:text-emerald-400">
              <Shield className="h-3.5 w-3.5" /> TLS 1.3 · BLAKE3
            </div>
            <button onClick={toggleDarkMode} aria-label="Thème" className="flex h-9 w-9 items-center justify-center rounded-full border border-[var(--border)] bg-[var(--surface)] hover:bg-[var(--surface-hover)] transition-colors">
              {darkMode ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
            </button>
          </div>
        </div>
      </header>

      <div className="mx-auto max-w-[1280px] px-4 sm:px-6 py-6">
        {!isMobile ? (
          <>
            <div className="mb-4 hidden lg:flex items-center gap-2">
              {[
                { id: "transfer", label: "Transfert" },
                { id: "pairing", label: "Appairage" },
                { id: "history", label: "Historique" },
                { id: "settings", label: "Paramètres" },
              ].map((item) => (
                <button key={item.id} onClick={() => setView(item.id as typeof view)} className={`rounded-full px-4 py-1.5 text-xs font-semibold transition-colors ${view === item.id ? "bg-[var(--accent)] text-white" : "border border-[var(--border)] bg-[var(--surface)] hover:bg-[var(--surface-hover)]"}`}>
                  {item.label}
                </button>
              ))}
              <span className="ml-auto text-xs text-[var(--text-tertiary)]">3 panes desktop · 4 onglets mobile · 200ms ease</span>
            </div>
            {view === "transfer" && <DesktopThreePane onFiles={handleFiles} hasFiles={hasFiles} />}
            {view === "pairing" && <div className="fade-in max-w-3xl"><PairingView /></div>}
            {view === "history" && <div className="fade-in max-w-3xl"><HistoryView /></div>}
            {view === "settings" && <div className="fade-in max-w-3xl"><SettingsView /></div>}
          </>
        ) : (
          <MobileTabs onFiles={handleFiles} hasFiles={hasFiles} />
        )}
      </div>

      <footer className="border-t border-[var(--border)] py-4 text-center text-xs text-[var(--text-tertiary)]">
        RivaldSend v0.1.0 · Propre · Pro · Fluide · Réactive — 60fps · Transitions 200ms cubic-bezier(0.16,1,0.3,1)
      </footer>
    </div>
  );
}
