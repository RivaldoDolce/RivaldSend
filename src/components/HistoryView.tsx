import { useRef } from "react";
import { History, RotateCcw, File } from "lucide-react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useHistoryStore } from "../stores/useHistoryStore";
import type { HistoryEntry } from "../types";

interface Props {
  direction?: "sent" | "received";
}

const VIRTUALIZE_THRESHOLD = 60;

function HistoryRow({ entry }: { entry: HistoryEntry }) {
  return (
    <div className="flex items-center gap-3 rounded-xl border border-[var(--border)] bg-[var(--surface)] p-3">
      <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-[var(--accent-light)]">
        <File className="h-5 w-5 text-[var(--accent)]" />
      </div>
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium truncate">{entry.fileName}</p>
        <p className="text-xs text-[var(--text-secondary)]">
          {entry.direction === "sent" ? "Envoyé à" : "Reçu de"} {entry.peerName}{" "}
          · {(entry.size / 1024 / 1024).toFixed(1)} Mo
        </p>
      </div>
      <button
        aria-label="Renvoyer"
        className="rounded-full p-2 hover:bg-[var(--surface-hover)]"
      >
        <RotateCcw className="h-4 w-4 text-[var(--text-secondary)]" />
      </button>
    </div>
  );
}

export function HistoryView({ direction }: Props) {
  const entries = useHistoryStore((s) => s.entries);
  const filtered = direction
    ? entries.filter((h) => h.direction === direction)
    : entries;

  const parentRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: filtered.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 72,
    overscan: 5,
  });

  if (filtered.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center rounded-2xl bg-[var(--surface)] border border-[var(--border)] p-10 text-center">
        <History className="h-10 w-10 text-[var(--text-secondary)]" />
        <p className="mt-3 text-sm font-medium">
          {direction === "sent"
            ? "Aucun envoi"
            : direction === "received"
              ? "Aucun reçu"
              : "Aucun historique"}
        </p>
        <p className="text-xs text-[var(--text-secondary)]">
          {direction === "sent"
            ? "Vos envois apparaîtront ici"
            : direction === "received"
              ? "Vos réceptions apparaîtront ici"
              : "Vos transferts apparaîtront ici"}
        </p>
      </div>
    );
  }

  if (filtered.length <= VIRTUALIZE_THRESHOLD) {
    return (
      <div className="space-y-2">
        {filtered.map((h) => (
          <HistoryRow key={h.id} entry={h} />
        ))}
      </div>
    );
  }

  return (
    <div ref={parentRef} className="max-h-[600px] overflow-y-auto pr-1">
      <div
        style={{ height: virtualizer.getTotalSize(), position: "relative" }}
      >
        {virtualizer.getVirtualItems().map((virtualRow) => {
          const entry = filtered[virtualRow.index];
          if (!entry) return null;
          return (
            <div
              key={entry.id}
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: "100%",
                transform: `translateY(${virtualRow.start}px)`,
                paddingBottom: 8,
              }}
            >
              <HistoryRow entry={entry} />
            </div>
          );
        })}
      </div>
    </div>
  );
}
