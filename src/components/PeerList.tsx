import { MonitorSmartphone, Smartphone, ShieldCheck, ShieldAlert, Search, Globe } from "lucide-react";
import { useTranslation } from "react-i18next";
import { usePeersStore } from "../stores/usePeersStore";

export function PeerList() {
  const { t } = useTranslation();
  const peers = usePeersStore((s) => s.peers);
  const selectedPeerId = usePeersStore((s) => s.selectedPeerId);
  const selectPeer = usePeersStore((s) => s.selectPeer);

  if (peers.length === 0) {
    return (
      <div className="card-premium flex flex-col items-center justify-center rounded-[24px] p-8 text-center">
        <div className="relative">
          <img src="/assets/empty-no-peers.webp" alt="" width="112" height="112" decoding="async" loading="lazy" className="h-28 w-28 object-contain opacity-90" onError={(e) => ((e.target as HTMLImageElement).style.display = "none")} />
          <div className="absolute -right-2 -top-2 flex h-7 w-7 items-center justify-center rounded-full bg-amber-100 text-amber-600 dark:bg-amber-500/20 dark:text-amber-300 border border-amber-500/20">
            <Search className="h-4 w-4" />
          </div>
        </div>
        <p className="mt-4 text-sm font-semibold text-[var(--text-primary)]">{t("noPeers")}</p>
        <p className="mt-1 max-w-xs text-xs leading-relaxed text-[var(--text-secondary)]">{t("noPeersDesc")}</p>
        <div className="mt-4 flex items-center gap-2 text-xs text-[var(--text-tertiary)]">
          mDNS + QR + IP manuelle
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col min-w-0 h-full flex-1 min-h-0">
      <div className="flex items-center justify-between px-1 pb-2 shrink-0">
        <h3 className="text-xs font-bold tracking-widest uppercase text-[var(--text-secondary)]">{t("peers")} · {peers.length}</h3>
        <span className="text-[11px] text-[var(--text-tertiary)]">{peers.length} en ligne</span>
      </div>
      <div className="grid gap-2.5 overflow-y-auto overflow-x-hidden overscroll-contain flex-1 min-h-0 pr-1 -mr-1 p-1 content-start">
        {peers.map((p) => {
          const active = selectedPeerId === p.id;
          return (
            <button
              key={p.id}
              onClick={() => selectPeer(p.id)}
              className={`group relative flex items-center gap-3 rounded-[16px] border p-3 text-left transition-all min-w-0 overflow-hidden w-full ${active ? "border-[var(--accent)] bg-[var(--accent-light)] shadow-md" : "card-premium hover:border-[var(--accent)]/30"}`}
            >
              {active && <div className="absolute left-0 top-1/2 h-7 w-1 -translate-y-1/2 rounded-full bg-[var(--accent)]" />}
              <div className={`flex h-10 w-10 shrink-0 items-center justify-center rounded-[12px] border ${active ? "bg-white dark:bg-[var(--surface-elevated)] border-[var(--accent)]/20 text-[var(--accent)]" : "bg-[var(--surface-hover)] border-[var(--border)] text-[var(--text-secondary)] group-hover:bg-white dark:group-hover:bg-[var(--surface-elevated)]"}`}>
                {p.platform === "android" || p.platform === "ios" ? <Smartphone className="h-5 w-5" strokeWidth={1.5} /> : p.platform === "unknown" ? <Globe className="h-5 w-5" strokeWidth={1.5} /> : <MonitorSmartphone className="h-5 w-5" strokeWidth={1.5} />}
              </div>
              <div className="flex-1 min-w-0 overflow-hidden">
                <p className="text-sm font-semibold text-[var(--text-primary)] truncate leading-tight">{p.name}</p>
                <p className="mono text-[11px] leading-tight text-[var(--text-secondary)] truncate">{p.ip}:{p.port} · {p.fingerprintShort}</p>
                <p className="text-[11px] leading-tight text-[var(--text-tertiary)] truncate capitalize">{p.status}</p>
              </div>
              <div className="flex shrink-0 flex-col items-end gap-1">
                {p.trusted ? <ShieldCheck className="h-4 w-4 text-[var(--success)]" /> : <ShieldAlert className="h-4 w-4 text-amber-500" />}
                <span className={`rounded-full px-2 py-0.5 text-[11px] font-semibold leading-none ${p.trusted ? "bg-emerald-500 text-white" : "bg-amber-500 text-white"}`}>{p.trusted ? "Pairé" : "Nouveau"}</span>
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
}
