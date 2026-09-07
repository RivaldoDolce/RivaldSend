import { usePeersStore } from "../stores/usePeersStore";
import { useTransfersStore } from "../stores/useTransfersStore";
import { useProgressStore } from "../stores/useProgressStore";

const APP_VERSION = "0.3.0";

export function StatusBar() {
  const peers = usePeersStore((s) => s.peers);
  const transfers = useTransfersStore((s) => s.transfers);
  const byId = useProgressStore((s) => s.byId);
  const activeIds = transfers.filter((t) => t.status === "running").map((t) => t.id);
  const active = activeIds.length;
  const speed = activeIds.reduce((a, id) => a + (byId[id]?.speedBps ?? transfers.find((t) => t.id === id)?.speedBps ?? 0), 0);
  const mb = (speed / 1024 / 1024).toFixed(0);
  return (
    <div className="flex h-8 items-center justify-between border-t border-[var(--border)] bg-[var(--surface)] px-4 text-xs text-[var(--text-secondary)]">
      <span className="flex items-center gap-2">
        <span className="h-2 w-2 rounded-full bg-emerald-500" />
        Connecté — {peers.length} appareil{peers.length !== 1 ? "s" : ""} visible{peers.length !== 1 ? "s" : ""}
        {active > 0 && ` · ${active} transfert${active !== 1 ? "s" : ""} actif${active !== 1 ? "s" : ""}`}
      </span>
      <span className="hidden sm:flex items-center gap-3">
        {active > 0 && <span>↑ {mb} Mo/s</span>}
        <span>v{APP_VERSION}</span>
      </span>
    </div>
  );
}
