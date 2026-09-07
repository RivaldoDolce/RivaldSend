import { memo } from "react";
import { Zap, Clock3, HardDrive, Pause, X } from "lucide-react";
import { useTransfersStore } from "../stores/useTransfersStore";
import { useProgressStore } from "../stores/useProgressStore";
import { useDeviceContext } from "../hooks/useDeviceContext";
import { cancelTransfer, pauseTransfer } from "../lib/tauri-bridge";

function formatBytes(n: number): string {
  if (n < 1024) return `${n} o`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} Ko`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} Mo`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} Go`;
}
function formatEta(secs: number): string {
  if (!secs || secs < 0) return "—";
  if (secs < 60) return `${secs}s`;
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return `${m}m ${String(s).padStart(2, "0")}s`;
}

const CircularProgress = memo(function CircularProgress({ pct, size = 120 }: { pct: number; size?: number }) {
  const r = 52;
  const c = 2 * Math.PI * r;
  const offset = c * (1 - pct / 100);
  return (
    <div className="relative flex items-center justify-center" style={{ width: size, height: size }}>
      <svg width={size} height={size} className="-rotate-90">
        <circle cx={size / 2} cy={size / 2} r={r} stroke="var(--border)" strokeWidth={8} fill="none" />
        <circle cx={size / 2} cy={size / 2} r={r} stroke="var(--accent)" strokeWidth={8} fill="none" strokeLinecap="round" strokeDasharray={c} strokeDashoffset={offset} style={{ transition: "stroke-dashoffset .3s" }} />
      </svg>
      <span className="absolute text-lg font-bold text-[var(--text-primary)]">{pct.toFixed(0)}%</span>
    </div>
  );
});

const TransferRow = memo(function TransferRow({ id }: { id: string }) {
  const tr = useTransfersStore((s) => s.transfers.find((t) => t.id === id));
  const progress = useProgressStore((s) => s.byId[id]);
  const { isMobile } = useDeviceContext();

  if (!tr) return null;

  const bytesDone = progress?.bytesDone ?? tr.bytesDone;
  const speedBps = progress?.speedBps ?? tr.speedBps;
  const etaSecs = progress?.etaSecs ?? tr.etaSecs;
  const pct = tr.totalBytes > 0 ? (bytesDone / tr.totalBytes) * 100 : 0;

  const chunks = tr.totalBytes > 0 ? Math.ceil(tr.totalBytes / (4 * 1024 * 1024)) : 0;
  const chunksDone = tr.totalBytes > 0 ? Math.floor((pct / 100) * chunks) : 0;

  if (isMobile) {
    return (
      <div className="card-premium flex flex-col items-center gap-4 rounded-[20px] p-6 text-center">
        <CircularProgress pct={pct} />
        <div className="min-w-0 w-full">
          <p className="text-sm font-semibold truncate text-[var(--text-primary)]">{tr.files[0]?.path ?? tr.id}</p>
          <p className="text-xs text-[var(--text-secondary)]">{formatBytes(speedBps)}/s · {formatBytes(bytesDone)} / {formatBytes(tr.totalBytes)}</p>
          <p className="mt-1 text-xs text-[var(--text-tertiary)]">Chunks {chunksDone}/{chunks} · {formatEta(etaSecs)} restant · TLS 1.3</p>
        </div>
        <div className="flex gap-3">
          <button onClick={() => pauseTransfer(tr.id).catch(console.error)} className="flex h-10 w-10 items-center justify-center rounded-full border border-[var(--border)] hover:bg-[var(--surface-hover)]" aria-label="Pause"><Pause className="h-4 w-4" /></button>
          <button onClick={() => cancelTransfer(tr.id).catch(console.error)} className="flex h-10 w-10 items-center justify-center rounded-full bg-red-500 text-white hover:bg-red-600" aria-label="Annuler"><X className="h-4 w-4" /></button>
        </div>
        <p className="text-xs text-[var(--text-tertiary)]">🔒 Chiffré · reprise possible</p>
      </div>
    );
  }

  return (
    <div className="card-premium rounded-[20px] p-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3 min-w-0">
          <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-[var(--accent-light)] text-[var(--accent)]">
            <HardDrive className="h-5 w-5" />
          </div>
          <div className="min-w-0">
            <p className="text-sm font-semibold truncate text-[var(--text-primary)]">{tr.files[0]?.path ?? tr.id}</p>
            <p className="text-xs text-[var(--text-secondary)]">{tr.files.length} fichier(s) - {formatBytes(tr.totalBytes)}</p>
          </div>
        </div>
        <span className="shrink-0 rounded-full bg-[var(--accent)] px-3 py-1 text-xs font-bold text-white">{pct.toFixed(1)}%</span>
      </div>
      <div className="mt-4 h-2.5 overflow-hidden rounded-full bg-[var(--background)] p-1">
        <div className="relative h-full overflow-hidden rounded-full bg-[var(--accent)]" style={{ width: `${pct}%`, transition: "width .3s" }}>
          <div className="shimmer absolute inset-0" />
        </div>
      </div>
      <div className="mt-3 flex flex-wrap justify-between gap-2 text-xs">
        <span className="inline-flex items-center gap-1.5 text-[var(--text-secondary)]">
          <Zap className="h-3.5 w-3.5 text-[var(--accent)]" /> {formatBytes(speedBps)}/s
        </span>
        <span className="inline-flex items-center gap-1.5 text-[var(--text-secondary)]">
          <Clock3 className="h-3.5 w-3.5" /> {formatEta(etaSecs)} · {formatBytes(bytesDone)} / {formatBytes(tr.totalBytes)}
        </span>
        <span className="text-[var(--text-tertiary)]">Chunks {chunksDone}/{chunks} · TLS 1.3</span>
        <span className="rounded-full bg-amber-100 px-2 py-0.5 text-xs font-medium text-amber-700 capitalize">{tr.status}</span>
      </div>
    </div>
  );
});

export function ProgressView() {
  const transferIds = useTransfersStore((s) => s.transfers.map((t) => t.id));
  if (transferIds.length === 0) return null;
  return (
    <div className="space-y-3 fade-in">
      <h3 className="text-xs font-bold tracking-widest uppercase text-[var(--text-secondary)]">Transferts en cours</h3>
      {transferIds.map((id) => (
        <TransferRow key={id} id={id} />
      ))}
    </div>
  );
}
