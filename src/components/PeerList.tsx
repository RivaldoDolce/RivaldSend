import { MonitorSmartphone, Smartphone, ShieldCheck, ShieldAlert, Wifi, Search } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useAppStore } from "../stores/appStore";

export function PeerList() {
  const { t } = useTranslation();
  const { peers, selectedPeerId, selectPeer } = useAppStore();

  if (peers.length === 0) {
    return (
      <div className="card-premium flex flex-col items-center justify-center rounded-[24px] p-8 text-center">
        <div className="relative">
          <img src="/assets/Images/empty-no-peers.png" alt="" className="h-28 w-28 object-contain opacity-90" onError={(e) => ((e.target as HTMLImageElement).style.display = "none")} />
          <div className="absolute -right-2 -top-2 flex h-7 w-7 items-center justify-center rounded-full bg-amber-100 text-amber-600">
            <Search className="h-4 w-4" />
          </div>
        </div>
        <p className="mt-4 text-sm font-semibold text-[var(--text-primary)]">{t("noPeers")}</p>
        <p className="mt-1 max-w-xs text-xs leading-relaxed text-[var(--text-secondary)]">{t("noPeersDesc")}</p>
        <div className="mt-4 flex items-center gap-2 text-xs text-[var(--text-tertiary)]">
          <Wifi className="h-3.5 w-3.5" /> mDNS + QR + IP manuelle
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-xs font-bold tracking-widest uppercase text-[var(--text-secondary)]">{t("peers")} · {peers.length}</h3>
        <span className="rounded-full bg-green-500/10 px-2.5 py-1 text-xs font-medium text-green-600">LAN détecté</span>
      </div>
      <div className="grid gap-3">
        {peers.map((p) => {
          const active = selectedPeerId === p.id;
          return (
            <button
              key={p.id}
              onClick={() => selectPeer(p.id)}
              className={`group relative flex items-center gap-4 rounded-[20px] border p-4 text-left transition-all ${active ? "border-[var(--accent)] bg-[var(--accent-light)] shadow-md" : "card-premium hover:border-[var(--accent)]/30"}`}
            >
              {active && <div className="absolute left-0 top-1/2 h-8 w-1 -translate-y-1/2 rounded-full bg-[var(--accent)]" />}
              <div className={`flex h-12 w-12 shrink-0 items-center justify-center rounded-[14px] border ${active ? "bg-white border-[var(--accent)]/20 text-[var(--accent)]" : "bg-[var(--surface-hover)] border-[var(--border)] text-[var(--text-secondary)] group-hover:bg-white"}`}>
                {p.name.includes("Mac") ? <MonitorSmartphone className="h-6 w-6" strokeWidth={1.5} /> : <Smartphone className="h-6 w-6" strokeWidth={1.5} />}
              </div>
              <div className="flex-1 min-w-0">
                <p className="text-sm font-semibold text-[var(--text-primary)] truncate">{p.name}</p>
                <p className="mono text-xs text-[var(--text-secondary)]">{p.ip}:{p.port} · {p.fingerprintShort} · {p.status}</p>
              </div>
              <div className="flex shrink-0 items-center gap-2">
                {p.trusted ? <ShieldCheck className="h-5 w-5 text-[var(--success)]" /> : <ShieldAlert className="h-5 w-5 text-amber-500" />}
                <span className={`rounded-full px-3 py-1 text-xs font-semibold ${p.trusted ? "bg-emerald-500 text-white" : "bg-amber-500 text-white"}`}>{p.trusted ? "Pairé" : "Nouveau"}</span>
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
}
