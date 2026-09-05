import { Download, X } from "lucide-react";
import { useIncomingStore } from "../stores/useIncomingStore";
import { useSettingsStore } from "../stores/useSettingsStore";
import { acceptIncoming, rejectIncoming, pickFolder } from "../lib/tauri-bridge";
import { useToast } from "./toast/Toast";

function formatBytes(n: number): string {
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} Ko`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} Mo`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} Go`;
}

export function IncomingRequestToast() {
  const pending = useIncomingStore((s) => s.pending);
  const clear = useIncomingStore((s) => s.clear);
  const toast = useToast();

  if (!pending) return null;

  const handleAccept = async () => {
    const dir =
      useSettingsStore.getState().downloadDir || (await pickFolder());
    if (!dir) return;
    try {
      await acceptIncoming({ requestId: pending.requestId, targetDir: dir });
      toast.success("Transfert accepté", `${pending.peerName} envoie ${pending.files.length} fichier(s)`);
      clear();
    } catch (err) {
      console.error("[IncomingRequest] accept failed:", err);
      toast.error("Erreur", "Impossible d'accepter le transfert");
    }
  };

  const handleChooseDir = async () => {
    const dir = await pickFolder();
    if (!dir) return;
    try {
      await acceptIncoming({ requestId: pending.requestId, targetDir: dir });
      toast.success("Transfert accepté", `Dossier : ${dir}`);
      clear();
    } catch (err) {
      console.error("[IncomingRequest] accept with custom dir failed:", err);
      toast.error("Erreur", "Impossible d'accepter le transfert");
    }
  };

  const handleReject = async () => {
    try {
      await rejectIncoming(pending.requestId);
      toast.info("Transfert refusé");
    } catch (err) {
      console.error("[IncomingRequest] reject failed:", err);
    }
    clear();
  };

  return (
    <div className="fixed bottom-24 right-4 z-50 w-96 max-w-[calc(100vw-2rem)] animate-slide-in-right">
      <div className="rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-4 shadow-2xl">
        <div className="flex items-start gap-3">
          <div className="h-10 w-10 rounded-xl bg-[var(--accent-light)] flex items-center justify-center shrink-0">
            <Download className="h-5 w-5 text-[var(--accent)]" />
          </div>
          <div className="flex-1 min-w-0">
            <p className="font-semibold text-sm">{pending.peerName} veut envoyer</p>
            <p className="text-xs text-[var(--text-secondary)] truncate mt-1">
              {pending.files.length} fichier(s) · {formatBytes(pending.totalBytes)}
            </p>
          </div>
          <button
            onClick={handleReject}
            aria-label="Refuser le transfert"
            className="shrink-0 rounded-full p-1 hover:bg-[var(--surface-hover)]"
          >
            <X className="h-4 w-4 text-[var(--text-tertiary)]" />
          </button>
        </div>
        <div className="mt-4 flex gap-2">
          <button
            onClick={handleReject}
            className="flex-1 rounded-full border border-[var(--border)] px-4 py-2 text-xs font-medium hover:bg-[var(--surface-hover)]"
          >
            Refuser
          </button>
          <button
            onClick={handleChooseDir}
            className="flex-1 rounded-full border border-[var(--border)] px-4 py-2 text-xs font-medium hover:bg-[var(--surface-hover)]"
          >
            Choisir dossier
          </button>
          <button
            onClick={handleAccept}
            className="flex-1 rounded-full bg-[var(--accent)] px-4 py-2 text-xs font-semibold text-white hover:bg-[var(--accent-hover)]"
          >
            Accepter
          </button>
        </div>
      </div>
    </div>
  );
}
