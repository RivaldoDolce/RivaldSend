import { usePeersStore } from "../stores/usePeersStore";
import { useTransfersStore } from "../stores/useTransfersStore";
import { useToast } from "./toast/Toast";
import { MonitorSmartphone, Smartphone, X, Send } from "lucide-react";
import { startTransfer } from "../lib/tauri-bridge";

export function SendModal() {
  const showSendModal = usePeersStore((s) => s.showSendModal);
  const pendingFiles = usePeersStore((s) => s.pendingFiles);
  const closeSendModal = usePeersStore((s) => s.closeSendModal);
  const peers = usePeersStore((s) => s.peers);
  const selectedPeerId = usePeersStore((s) => s.selectedPeerId);
  const selectPeer = usePeersStore((s) => s.selectPeer);
  const addTransfer = useTransfersStore((s) => s.addTransfer);
  const toast = useToast();

  if (!showSendModal) return null;

  const totalSize = pendingFiles.reduce((s, f) => s + f.size, 0);
  const selectedPeer = peers.find((p) => p.id === selectedPeerId);

  const handleSend = async () => {
    if (!selectedPeer) return;
    const transferId = crypto.randomUUID();
    const transfer = {
      id: transferId,
      files: pendingFiles.map((f) => ({ path: f.path, size: f.size, blake3: "0".repeat(64) })),
      totalBytes: totalSize,
      bytesDone: 0,
      speedBps: 0,
      etaSecs: 0,
      status: "queued" as const,
      peerId: selectedPeer.id,
      createdAt: new Date().toISOString(),
    };
    addTransfer(transfer);
    closeSendModal();
    toast.info("Transfert lancé", `Envoi vers ${selectedPeer.name}...`);

    try {
      await startTransfer({
        peerId: selectedPeer.id,
        filePaths: pendingFiles.map((f) => f.path),
      });
    } catch {
      useTransfersStore.getState().updateTransfer(transferId, { status: "failed" });
      toast.error("Échec", "Impossible de démarrer le transfert");
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm">
      <div className="w-full max-w-md rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-6 shadow-xl animate-scale-in">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-semibold">Envoyer à…</h3>
          <button onClick={closeSendModal} className="rounded-full p-1 hover:bg-[var(--surface-hover)]">
            <X className="h-4 w-4" />
          </button>
        </div>

        <p className="mt-2 text-xs text-[var(--text-secondary)]">
          {pendingFiles.length} fichier(s) · {(totalSize / 1024 / 1024).toFixed(1)} Mo
        </p>

        <div className="mt-4 space-y-2 max-h-80 overflow-y-auto">
          {peers.length === 0 && (
            <p className="text-center text-xs text-[var(--text-secondary)] py-4">
              Aucun appareil découvert. Vérifiez que les deux appareils sont sur le même réseau.
            </p>
          )}
          {peers.map((p) => {
            const active = selectedPeerId === p.id;
            return (
              <button
                key={p.id}
                onClick={() => selectPeer(p.id)}
                className={`w-full flex items-center gap-3 rounded-xl border p-3 text-left transition-all ${active ? "border-[var(--accent)] bg-[var(--accent-light)]" : "border-[var(--border)] hover:border-[var(--accent)]/30"}`}
              >
                <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-[var(--surface-hover)]">
                  {p.name.includes("Mac") ? <MonitorSmartphone className="h-5 w-5" strokeWidth={1.5} /> : <Smartphone className="h-5 w-5" strokeWidth={1.5} />}
                </div>
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium truncate">{p.name}</p>
                  <p className="text-xs text-[var(--text-secondary)]">{p.ip}:{p.port}</p>
                </div>
                {active && <span className="text-xs font-bold text-[var(--accent)]">Sélectionné</span>}
              </button>
            );
          })}
        </div>

        <div className="mt-5 flex justify-end gap-2">
          <button onClick={closeSendModal} className="rounded-full border border-[var(--border)] px-4 py-2 text-sm font-medium hover:bg-[var(--surface-hover)]">
            Annuler
          </button>
          <button
            onClick={handleSend}
            disabled={!selectedPeer}
            className="inline-flex items-center gap-2 rounded-full bg-[var(--accent)] px-5 py-2 text-sm font-semibold text-white shadow-md disabled:opacity-40 disabled:cursor-not-allowed hover:bg-[var(--accent-hover)]"
          >
            <Send className="h-4 w-4" /> Envoyer
          </button>
        </div>
      </div>
    </div>
  );
}
