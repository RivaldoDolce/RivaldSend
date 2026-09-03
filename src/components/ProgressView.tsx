import { useAppStore } from "../stores/appStore";

function formatBytes(n: number): string {
  if (n < 1024) return `${n} o`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} Ko`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} Mo`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} Go`;
}

function formatSpeed(bps: number): string {
  return `${formatBytes(bps)}/s`;
}

export function ProgressView() {
  const { transfers } = useAppStore();
  if (transfers.length === 0) return null;
  return (
    <div className="space-y-3">
      {transfers.map((tr) => {
        const pct = tr.totalBytes > 0 ? (tr.bytesDone / tr.totalBytes) * 100 : 0;
        return (
          <div key={tr.id} className="rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-4">
            <div className="flex items-center justify-between">
              <p className="text-sm font-medium text-[var(--text-primary)] truncate">{tr.files[0]?.path ?? tr.id}</p>
              <span className="text-xs font-medium text-[var(--accent)]">{pct.toFixed(1)}%</span>
            </div>
            <div className="mt-3 h-2 overflow-hidden rounded-full bg-[var(--background)]">
              <div className="h-full rounded-full bg-[var(--accent)] transition-all" style={{ width: `${pct}%` }} />
            </div>
            <div className="mt-2 flex justify-between text-xs text-[var(--text-secondary)]">
              <span>
                {formatBytes(tr.bytesDone)} / {formatBytes(tr.totalBytes)}
              </span>
              <span>
                {formatSpeed(tr.speedBps)} · ETA {tr.etaSecs}s
              </span>
            </div>
            <p className="mt-1 text-xs text-[var(--text-secondary)] capitalize">{tr.status}</p>
          </div>
        );
      })}
    </div>
  );
}
