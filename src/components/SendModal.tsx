import { useCallback } from "react";
import { usePeersStore } from "../stores/usePeersStore";
import { useTransfersStore } from "../stores/useTransfersStore";
import { useNavStore } from "../stores/useNavStore";
import { useToast } from "./toast/Toast";
import { useDeviceContext } from "../hooks/useDeviceContext";
import { MonitorSmartphone, Smartphone, Users, X, Send, QrCode } from "lucide-react";
import { startTransfer } from "../lib/tauri-bridge";

function SendModalContent({ onClose }: { onClose: () => void }) {
  const pendingFiles = usePeersStore((s) => s.pendingFiles);
  const peers = usePeersStore((s) => s.peers);
  const selectedPeerId = usePeersStore((s) => s.selectedPeerId);
  const selectPeer = usePeersStore((s) => s.selectPeer);
  const addTransfer = useTransfersStore((s) => s.addTransfer);
  const toast = useToast();

  const totalSize = pendingFiles.reduce((s, f) => s + f.size, 0);
  const selectedPeer = peers.find((p) => p.id === selectedPeerId);

  const handleSend = useCallback(async () => {
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
    onClose();

    try {
      await startTransfer({
        peerId: selectedPeer.id,
        filePaths: pendingFiles.map((f) => f.path),
      });
      toast.success("Transfert demarre", `Envoi vers ${selectedPeer.name}`);
    } catch (err) {
      const message =
        err instanceof Error ? err.message : String(err ?? "Erreur inconnue");
      console.error("[SendModal] startTransfer failed:", err);
      useTransfersStore
        .getState()
        .updateTransfer(transferId, { status: "failed", error: message });
      toast.error("Echec du demarrage", "Impossible de contacter l'appareil");
    }
  }, [selectedPeer, pendingFiles, totalSize, addTransfer, onClose, toast]);

  return (
    <>
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold">Envoyer a...</h3>
        <button onClick={onClose} aria-label="Fermer" className="rounded-full p-1 hover:bg-[var(--surface-hover)]">
          <X className="h-4 w-4" />
        </button>
      </div>

      <p className="mt-2 text-xs text-[var(--text-secondary)]">
        {pendingFiles.length} fichier(s)
        {totalSize > 0 ? ` - ${(totalSize / 1024 / 1024).toFixed(1)} Mo` : ""}
      </p>

      <div className="mt-4 space-y-2 max-h-80 overflow-y-auto">
        {peers.length === 0 ? (
          <div className="py-8 text-center">
            <Users className="mx-auto h-12 w-12 text-[var(--text-tertiary)]" />
            <p className="mt-4 text-sm font-medium">Aucun appareil trouve</p>
            <p className="mx-auto mt-2 max-w-xs text-xs leading-relaxed text-[var(--text-secondary)]">
              Assurez-vous que l'autre appareil a RivaldSend ouvert et
              est sur le meme reseau.
            </p>
            <button
              onClick={() => {
                onClose();
                useNavStore.getState().setView("pairing");
              }}
              className="mt-4 inline-flex items-center gap-2 rounded-full bg-[var(--accent)] px-5 py-2 text-xs font-semibold text-white hover:bg-[var(--accent-hover)]"
            >
              <QrCode className="h-4 w-4" /> Scanner un QR code
            </button>
          </div>
        ) : null}
        {peers.map((p) => {
          const active = selectedPeerId === p.id;
          return (
            <button
              key={p.id}
              onClick={() => selectPeer(p.id)}
              className={`flex w-full items-center gap-3 rounded-xl border p-3 text-left transition-all ${active ? "border-[var(--accent)] bg-[var(--accent-light)]" : "border-[var(--border)] hover:border-[var(--accent)]/30"}`}
            >
              <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-[var(--surface-hover)]">
                {p.platform === "android" || p.platform === "ios" ? (
                  <Smartphone className="h-5 w-5" strokeWidth={1.5} />
                ) : (
                  <MonitorSmartphone className="h-5 w-5" strokeWidth={1.5} />
                )}
              </div>
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium">{p.name}</p>
                <p className="text-xs text-[var(--text-secondary)]">{p.ip}:{p.port}</p>
              </div>
              {active && <span className="text-xs font-bold text-[var(--accent)]">Selectionne</span>}
            </button>
          );
        })}
      </div>

      <div className="mt-5 flex justify-end gap-2">
        <button onClick={onClose} className="rounded-full border border-[var(--border)] px-4 py-2 text-sm font-medium hover:bg-[var(--surface-hover)]">
          Annuler
        </button>
        <button
          onClick={handleSend}
          disabled={!selectedPeer}
          className="inline-flex items-center gap-2 rounded-full bg-[var(--accent)] px-5 py-2 text-sm font-semibold text-white shadow-md disabled:cursor-not-allowed disabled:opacity-40 hover:bg-[var(--accent-hover)]"
        >
          <Send className="h-4 w-4" /> Envoyer
        </button>
      </div>
    </>
  );
}

export function SendModal() {
  const showSendModal = usePeersStore((s) => s.showSendModal);
  const closeSendModal = usePeersStore((s) => s.closeSendModal);
  const { isMobile } = useDeviceContext();

  if (!showSendModal) return null;

  if (isMobile) {
    return (
      <div className="fixed inset-0 z-50" onClick={closeSendModal}>
        <div className="absolute inset-0 bg-black/40" />
        <div
          className="sheet open absolute bottom-0 left-0 right-0 border-t border-[var(--border)] bg-[var(--surface)] p-6 shadow-xl"
          onClick={(e) => e.stopPropagation()}
        >
          <div className="mx-auto mb-4 h-1 w-10 rounded-full bg-[var(--border-strong)]" />
          <SendModalContent onClose={closeSendModal} />
        </div>
      </div>
    );
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm" onClick={closeSendModal}>
      <div
        className="animate-scale-in w-full max-w-md rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-6 shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <SendModalContent onClose={closeSendModal} />
      </div>
    </div>
  );
}
