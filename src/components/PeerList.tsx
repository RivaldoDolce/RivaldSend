import { MonitorSmartphone, Smartphone, ShieldCheck, ShieldAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useAppStore } from "../stores/appStore";

export function PeerList() {
  const { t } = useTranslation();
  const { peers, selectedPeerId, selectPeer } = useAppStore();

  if (peers.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center rounded-2xl bg-[var(--surface)] p-8 border border-[var(--border)]">
        <img src="/assets/Images/empty-no-peers.png" alt="" className="h-24 opacity-60" onError={(e) => ((e.target as HTMLImageElement).style.display = "none")} />
        <p className="mt-4 text-sm font-medium text-[var(--text-primary)]">{t("noPeers")}</p>
        <p className="mt-1 text-xs text-[var(--text-secondary)] text-center max-w-xs">{t("noPeersDesc")}</p>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <h3 className="text-sm font-semibold text-[var(--text-primary)] tracking-wide uppercase">{t("peers")}</h3>
      <div className="grid gap-3">
        {peers.map((p) => (
          <button
            key={p.id}
            onClick={() => selectPeer(p.id)}
            className={`flex items-center gap-4 rounded-2xl border p-4 text-left transition-all ${selectedPeerId === p.id ? "border-[var(--accent)] bg-[var(--accent-light)]" : "border-[var(--border)] bg-[var(--surface)] hover:bg-[var(--surface-hover)]"}`}
          >
            <div className="flex h-11 w-11 items-center justify-center rounded-xl bg-[var(--background)] border border-[var(--border)]">
              {p.name.includes("Mac") ? <MonitorSmartphone className="h-5 w-5 text-[var(--text-secondary)]" /> : <Smartphone className="h-5 w-5 text-[var(--text-secondary)]" />}
            </div>
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium text-[var(--text-primary)] truncate">{p.name}</p>
              <p className="text-xs text-[var(--text-secondary)] mono">
                {p.ip}:{p.port} · {p.fingerprintShort}
              </p>
            </div>
            {p.trusted ? <ShieldCheck className="h-5 w-5 text-[var(--success)] shrink-0" /> : <ShieldAlert className="h-5 w-5 text-[var(--warning)] shrink-0" />}
            <span className={`shrink-0 rounded-full px-2.5 py-1 text-xs font-medium ${p.trusted ? "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400" : "bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400"}`}>
              {p.trusted ? "Pairé" : "Inconnu"}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}
