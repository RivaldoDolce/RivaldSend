import { History, RotateCcw, File } from "lucide-react";
import { useAppStore } from "../stores/appStore";

export function HistoryView() {
  const { history } = useAppStore();
  if (history.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center rounded-2xl bg-[var(--surface)] border border-[var(--border)] p-10 text-center">
        <History className="h-10 w-10 text-[var(--text-secondary)]" />
        <p className="mt-3 text-sm font-medium">Aucun historique</p>
        <p className="text-xs text-[var(--text-secondary)]">Vos transferts apparaîtront ici</p>
      </div>
    );
  }
  return (
    <div className="space-y-2">
      {history.map((h) => (
        <div key={h.id} className="flex items-center gap-3 rounded-xl border border-[var(--border)] bg-[var(--surface)] p-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-[var(--accent-light)]">
            <File className="h-5 w-5 text-[var(--accent)]" />
          </div>
          <div className="flex-1 min-w-0">
            <p className="text-sm font-medium truncate">{h.fileName}</p>
            <p className="text-xs text-[var(--text-secondary)]">
              {h.direction === "sent" ? "Envoyé à" : "Reçu de"} {h.peerName} · {(h.size / 1024 / 1024).toFixed(1)} Mo
            </p>
          </div>
          <button className="rounded-full p-2 hover:bg-[var(--surface-hover)]">
            <RotateCcw className="h-4 w-4 text-[var(--text-secondary)]" />
          </button>
        </div>
      ))}
    </div>
  );
}
