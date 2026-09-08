import { useEffect, useState, useCallback, memo } from "react";
import { Send, History, Settings, Shield, Sun, Moon, Home, Inbox, Wifi, Pause, X, FileText } from "lucide-react";
import { cancelTransfer, pauseTransfer, resumeTransfer } from "./lib/tauri-bridge";
import { checkFirewall } from "./lib/tauri-bridge";
import { useProgressStore } from "./stores/useProgressStore";
import { DropZone } from "./components/DropZone";
import { PeerList } from "./components/PeerList";
import { ProgressView } from "./components/ProgressView";
import { PairingView } from "./components/PairingView";
import { DiscoveryView } from "./components/DiscoveryView";
import { HistoryView } from "./components/HistoryView";
import { SettingsView } from "./components/SettingsView";
import { SendModal } from "./components/SendModal";
import { Onboarding } from "./components/Onboarding";
import { IncomingRequestToast } from "./components/IncomingRequestToast";
import { ToastProvider } from "./components/toast/Toast";
import { StatusBar } from "./components/StatusBar";
import { CommandPalette } from "./components/CommandPalette";
import { useNavStore } from "./stores/useNavStore";
import { usePeersStore } from "./stores/usePeersStore";
import { useTransfersStore } from "./stores/useTransfersStore";
import { useSettingsStore } from "./stores/useSettingsStore";
import { useTauriEvents } from "./hooks/useTauriEvents";
import { useKeyboardShortcuts } from "./hooks/useKeyboardShortcuts";
import { useDeviceContext } from "./hooks/useDeviceContext";
import "./i18n";

const APP_VERSION = "0.3.0";

function formatEta(secs: number): string {
  if (!secs || secs < 0) return "—";
  if (secs < 60) return `${secs}s`;
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return `${m}m ${String(s).padStart(2, "0")}s`;
}
const SidebarNav = memo(function SidebarNav() {
  const view = useNavStore((s) => s.view);
  const setView = useNavStore((s) => s.setView);
  const items = [
    { id: "transfer", label: "Transfert", icon: Send },
    { id: "discovery", label: "Découverte", icon: Wifi },
    { id: "pairing", label: "Appairage", icon: Shield },
    { id: "history", label: "Historique", icon: History },
    { id: "settings", label: "Paramètres", icon: Settings },
  ] as const;
  return (
    <nav className="flex flex-col gap-2 rounded-[20px] border border-[var(--border)] bg-[var(--surface)] p-2 shadow-sm w-full">
      {items.map((item) => {
        const active = view === item.id;
        return (
          <button
            key={item.id}
            onClick={() => setView(item.id as typeof view)}
            title={item.label}
            className={`flex items-center gap-3 rounded-[14px] px-3 py-3 text-left transition-colors ${active ? "bg-[var(--accent)] text-white shadow-sm" : "hover:bg-[var(--surface-hover)]"}`}
          >
            <item.icon className="h-5 w-5 shrink-0" strokeWidth={1.75} />
            <span className="hidden xl:inline text-sm font-semibold">{item.label}</span>
          </button>
        );
      })}
    </nav>
  );
});

const MobileBottomNav = memo(function MobileBottomNav() {
  const mobileTab = useNavStore((s) => s.mobileTab);
  const setMobileTab = useNavStore((s) => s.setMobileTab);
  const tabs = [
    { id: "home", label: "Accueil", icon: Home },
    { id: "discovery", label: "Découverte", icon: Wifi },
    { id: "send", label: "Envoyer", icon: Send },
    { id: "received", label: "Reçus", icon: Inbox },
    { id: "settings", label: "Param.", icon: Settings },
  ] as const;
  return (
    <nav className="fixed bottom-0 inset-x-0 z-30 border-t border-[var(--border)] bg-[var(--surface)]/95 safe-area-bottom">
      <div className="mx-auto grid max-w-md grid-cols-5 gap-1 px-2 py-2">
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

const TransferRowInline = memo(function TransferRowInline({ id, selectedId, onSelect }: { id: string; selectedId: string | null; onSelect: (id: string) => void }) {
  const tr = useTransfersStore((s) => s.transfers.find((t) => t.id === id));
  const prog = useProgressStore((s) => s.byId[id]);
  if (!tr) return null;
  const bytesDone = prog?.bytesDone ?? tr.bytesDone;
  const speedBps = prog?.speedBps ?? tr.speedBps;
  const pct = tr.totalBytes > 0 ? (bytesDone / tr.totalBytes) * 100 : 0;
  const isActive = tr.status === "running" || tr.status === "queued";
  return (
    <button
      onClick={() => onSelect(tr.id)}
      className={`card-premium w-full rounded-[20px] p-4 text-left ${selectedId === tr.id ? "ring-2 ring-[var(--accent)]" : ""}`}
    >
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3 min-w-0">
          <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-[var(--accent-light)] text-[var(--accent)]">
            <FileText className="h-6 w-6" />
          </div>
          <div className="min-w-0">
            <p className="text-sm font-semibold truncate">{tr.files[0]?.path?.split(/[\\/]/).pop() ?? tr.id}</p>
            <div className="mt-1 h-1.5 w-40 overflow-hidden rounded-full bg-[var(--background)]">
              <div className="h-full rounded-full bg-[var(--accent)]" style={{ width: `${pct}%`, transition: "width .3s" }} />
            </div>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-xs font-medium text-[var(--text-secondary)]">{(speedBps / 1024 / 1024).toFixed(1)} Mo/s</span>
          {isActive && (
            <div className="flex gap-1" onClick={(e) => e.stopPropagation()}>
              <button aria-label="Pause" onClick={() => pauseTransfer(tr.id).catch(console.error)} className="flex h-6 w-6 items-center justify-center rounded-full border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]">
                <Pause className="h-3 w-3" />
              </button>
              <button aria-label="Annuler" onClick={() => cancelTransfer(tr.id).catch(console.error)} className="flex h-6 w-6 items-center justify-center rounded-full border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--surface-hover)]">
                <X className="h-3 w-3" />
              </button>
            </div>
          )}
        </div>
      </div>
    </button>
  );
});

const TransferDetails = memo(function TransferDetails({ id }: { id: string }) {
  const tr = useTransfersStore((s) => s.transfers.find((t) => t.id === id));
  const prog = useProgressStore((s) => s.byId[id]);
  if (!tr) return null;
  const speedBps = prog?.speedBps ?? tr.speedBps;
  const etaSecs = prog?.etaSecs ?? tr.etaSecs;
  const bytesDone = prog?.bytesDone ?? tr.bytesDone;
  const chunksTotal = Math.ceil(tr.totalBytes / (4 * 1024 * 1024)) || 1;
  const chunksDone = Math.min(chunksTotal, Math.ceil(bytesDone / (4 * 1024 * 1024)));
  return (
    <>
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-bold">Détails</h3>
      </div>
      <p className="mono mt-2 text-xs break-all">{tr.files[0]?.path}</p>
      <div className="mt-3 space-y-2 text-xs">
        <div className="flex justify-between"><span className="text-[var(--text-secondary)]">Vitesse</span><span className="font-medium">{(speedBps / 1024 / 1024).toFixed(1)} Mo/s</span></div>
        <div className="flex justify-between"><span className="text-[var(--text-secondary)]">Temps restant</span><span className="font-medium">{formatEta(etaSecs)}</span></div>
        <div className="flex justify-between"><span className="text-[var(--text-secondary)]">Chunks</span><span className="font-medium">{chunksDone} / {chunksTotal}</span></div>
        <div className="flex justify-between"><span className="text-[var(--text-secondary)]">Chiffrement</span><span className="text-emerald-600 dark:text-emerald-400">TLS 1.3 · BLAKE3</span></div>
      </div>
      <div className="mt-4 flex gap-2">
        {tr.status === "paused" ? (
          <button onClick={() => resumeTransfer(tr.id).catch(console.error)} className="flex-1 rounded-full border border-[var(--border)] px-4 py-2 text-xs font-medium hover:bg-[var(--surface-hover)]">Reprendre</button>
        ) : (
          <button onClick={() => pauseTransfer(tr.id).catch(console.error)} className="flex-1 rounded-full border border-[var(--border)] px-4 py-2 text-xs font-medium hover:bg-[var(--surface-hover)]"><Pause className="mr-1 inline h-3 w-3" /> Pause</button>
        )}
        <button onClick={() => cancelTransfer(tr.id).catch(console.error)} className="flex-1 rounded-full border border-[var(--border)] px-4 py-2 text-xs font-medium text-[var(--error)] hover:bg-red-50 dark:hover:bg-red-950/30"><X className="mr-1 inline h-3 w-3" /> Annuler</button>
      </div>
    </>
  );
});

const LeftPane = memo(function LeftPane() {
  const peerCount = usePeersStore((s) => s.peers.length);
  return (
    <div className="min-w-0 md:sticky md:top-[88px] self-start">
      <div className="rounded-[20px] border border-[var(--border)] bg-[var(--surface)] shadow-sm overflow-hidden flex flex-col max-h-[min(72vh,640px)] md:max-h-[calc(100vh-120px)]">
        <div className="flex items-center justify-between px-4 py-3 border-b border-[var(--border)] shrink-0 bg-[var(--surface)]">
          <h2 className="text-xs font-bold tracking-widest uppercase text-[var(--text-secondary)]">Appareils</h2>
          <span className="text-xs font-medium text-[var(--text-tertiary)]">{peerCount ? `${peerCount}` : ""}</span>
        </div>
        <div className="p-3 overflow-hidden flex-1 min-h-0 flex flex-col">
          <PeerList />
        </div>
      </div>
    </div>
  );
});

const TransferThreePane = memo(function TransferThreePane({
  onFiles,
  onPaths,
}: {
  onFiles: (f: File[]) => void;
  onPaths: (paths: string[]) => void;
}) {
  const selectedTransferId = useTransfersStore((s) => s.selectedTransferId);
  const transferIds = useTransfersStore((s) => s.transfers.map((t) => t.id));
  const selectTransfer = useTransfersStore((s) => s.selectTransfer);
  const selectedTransfer = useTransfersStore((s) => s.transfers.find((t) => t.id === selectedTransferId));
  const selected = selectedTransfer ? selectedTransfer.id : null;
  const selectedIsActive = selectedTransfer?.status === "running" || selectedTransfer?.status === "paused";
  return (
    <div className="grid gap-6 grid-cols-1 md:grid-cols-[320px_minmax(0,1fr)] xl:grid-cols-[320px_minmax(0,1fr)_360px] items-start">
      <LeftPane />

      <div className="space-y-4 min-w-0 overflow-hidden">
        <div className="flex items-center gap-2 rounded-full border border-emerald-200 bg-emerald-50 px-3 py-1.5 text-xs font-medium text-emerald-700 dark:border-emerald-900 dark:bg-emerald-950 dark:text-emerald-300 overflow-hidden">
          <span className="h-2 w-2 rounded-full bg-emerald-500 animate-pulse shrink-0" /> <span className="truncate">Vos fichiers ne quittent jamais votre réseau local · Chiffré de bout en bout</span>
        </div>
        <DropZone onFilesSelected={onFiles} onPathsSelected={onPaths} />
        {transferIds.length > 0 ? (
          <div className="space-y-3 min-w-0">
            {transferIds.map((id) => (
              <TransferRowInline key={id} id={id} selectedId={selectedTransferId} onSelect={selectTransfer} />
            ))}
          </div>
        ) : (
          <div className="rounded-[20px] border border-dashed border-[var(--border-strong)] bg-[var(--surface)] p-6 text-center">
            <p className="text-sm font-medium">Aucun transfert</p>
            <p className="mt-1 text-xs text-[var(--text-secondary)]">Glisse un fichier — envoi en &lt;3s</p>
          </div>
        )}
        {selected && (
          <div className="xl:hidden rounded-[20px] border bg-[var(--surface)] shadow-sm p-4">
            {selectedIsActive ? <ProgressView transferId={selected} /> : <TransferDetails id={selected} />}
          </div>
        )}
      </div>

      <div className="hidden xl:block min-w-0 xl:sticky xl:top-[88px] self-start space-y-4">
        <div className={`rounded-[20px] border bg-[var(--surface)] shadow-sm overflow-hidden ${selected ? "p-4" : "border-dashed border-[var(--border-strong)] p-6 text-center"}`}>
          {selected ? (
            selectedIsActive ? <ProgressView transferId={selected} /> : <TransferDetails id={selected} />
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

function AppInner() {
  const darkMode = useSettingsStore((s) => s.darkMode);
  const view = useNavStore((s) => s.view);
  const mobileTab = useNavStore((s) => s.mobileTab);
  const [showOnboarding, setShowOnboarding] = useState(() => {
    return !localStorage.getItem("rivaldsend-onboarded");
  });

  useTauriEvents();
  useKeyboardShortcuts();

  useEffect(() => {
    document.documentElement.classList.toggle("dark", darkMode);
  }, [darkMode]);

  useEffect(() => {
    checkFirewall().catch((err) => {
      console.warn("[firewall] check unavailable:", err);
    });
  }, []);

  const handleFiles = useCallback(
    (files: File[]) => {
      usePeersStore.getState().openSendModal(files.map((f) => ({ path: f.name, size: f.size })));
    },
    []
  );

  const handlePaths = useCallback(
    (paths: string[]) => {
      usePeersStore.getState().openSendModal(paths.map((p) => ({ path: p, size: 0 })));
    },
    []
  );

  const handleOnboardingComplete = useCallback(() => {
    localStorage.setItem("rivaldsend-onboarded", "1");
    setShowOnboarding(false);
  }, []);

  const { isMobile } = useDeviceContext();
  const activeTransfer = useTransfersStore((s) => s.transfers.find((t) => t.id === s.selectedTransferId));
  const showProgressOverlay = isMobile && !!activeTransfer && (activeTransfer.status === "running" || activeTransfer.status === "paused");

  if (showOnboarding) {
    return <Onboarding onComplete={handleOnboardingComplete} />;
  }

  return (
    <div className="min-h-screen app-aurora text-[var(--text-primary)] antialiased">
      <header className="sticky top-0 z-20 border-b border-[var(--border)] bg-[var(--surface)]">
        <div className="mx-auto flex max-w-[1280px] items-center justify-between px-4 sm:px-6 py-3.5">
          <div className="flex items-center gap-3">
            <img src={darkMode ? "/assets/symbol-on-dark.webp" : "/assets/symbol-on-light.webp"} alt="RivaldSend" width="36" height="36" decoding="async" className="h-9 w-9 rounded-xl bg-white dark:bg-zinc-800 p-1.5 shadow-sm object-contain" onError={(e) => ((e.target as HTMLImageElement).style.display = "none")} />
            <div>
              <p className="text-[15px] font-extrabold tracking-tight leading-none">RivaldSend</p>
              <p className="hidden sm:block text-xs font-medium text-[var(--text-secondary)]">Pro · Réactive</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => {
                localStorage.removeItem("rivaldsend-onboarded");
                setShowOnboarding(true);
              }}
              className="text-xs text-[var(--text-tertiary)] hover:text-[var(--text-secondary)]"
            >
              Aide
            </button>
            <button onClick={useSettingsStore.getState().toggleDarkMode} aria-label="Thème" className="flex h-9 w-9 items-center justify-center rounded-full border border-[var(--border)] bg-[var(--surface)] hover:bg-[var(--surface-hover)] transition-colors">
              {darkMode ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
            </button>
          </div>
        </div>
      </header>

      <div className="mx-auto max-w-[1280px] px-4 sm:px-6 py-6">
        {isMobile ? (
          <div className="space-y-4 pb-20">
            {mobileTab === "home" && <TransferThreePane onFiles={handleFiles} onPaths={handlePaths} />}
            {mobileTab === "discovery" && <div className="fade-in"><DiscoveryView /></div>}
            {mobileTab === "send" && <div className="fade-in"><HistoryView direction="sent" /></div>}
            {mobileTab === "received" && <div className="fade-in"><HistoryView direction="received" /></div>}
            {mobileTab === "settings" && <div className="fade-in"><SettingsView /></div>}
          </div>
        ) : (
          <div className="grid grid-cols-1 gap-6 md:grid-cols-[64px_minmax(0,1fr)] xl:grid-cols-[240px_minmax(0,1fr)]">
            <div className="hidden md:block">
              <SidebarNav />
            </div>
            <main className="min-w-0">
              {view === "transfer" && <TransferThreePane onFiles={handleFiles} onPaths={handlePaths} />}
              {view === "discovery" && <div className="fade-in max-w-3xl"><DiscoveryView /></div>}
              {view === "pairing" && <div className="fade-in max-w-3xl"><PairingView /></div>}
              {view === "history" && <div className="fade-in max-w-3xl"><HistoryView /></div>}
              {view === "settings" && <div className="fade-in max-w-3xl"><SettingsView /></div>}
            </main>
          </div>
        )}
      </div>

      {isMobile && <MobileBottomNav />}
      {showProgressOverlay && activeTransfer && (
        <div className="fixed inset-0 z-40 overflow-y-auto bg-[var(--background)] px-4 py-6 pb-24">
          <div className="mx-auto max-w-md space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="text-sm font-bold">Transfert en cours</h2>
              <button onClick={() => useTransfersStore.getState().selectTransfer(null)} aria-label="Fermer" className="flex h-8 w-8 items-center justify-center rounded-full border border-[var(--border)] hover:bg-[var(--surface-hover)]">
                <X className="h-4 w-4" />
              </button>
            </div>
            <ProgressView transferId={activeTransfer.id} />
          </div>
        </div>
      )}
      <SendModal />
      <IncomingRequestToast />
      <CommandPalette />

      {!isMobile ? <StatusBar /> : (
        <footer className="border-t border-[var(--border)] py-3 text-center text-xs text-[var(--text-tertiary)] pb-[env(safe-area-inset-bottom)]">
          RivaldSend v{APP_VERSION}
        </footer>
      )}
    </div>
  );
}

export default function App() {
  return (
    <ToastProvider>
      <AppInner />
    </ToastProvider>
  );
}
