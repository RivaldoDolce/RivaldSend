import { useEffect } from "react";
import {
  onPeerDiscovered,
  onPeerLost,
  onTransferProgress,
  onTransferCompleted,
  onIncomingRequest,
  type TransferProgressEvent,
  type PeerDiscoveredEvent,
  type IncomingRequestEvent,
} from "../lib/tauri-bridge";
import { usePeersStore } from "../stores/usePeersStore";
import { useTransfersStore } from "../stores/useTransfersStore";
import { useHistoryStore } from "../stores/useHistoryStore";
import { useIncomingStore } from "../stores/useIncomingStore";
import type { Peer } from "../types";

export function useTauriEvents() {
  const addPeer = usePeersStore((s) => s.addPeer);
  const removePeer = usePeersStore((s) => s.removePeer);
  const updateTransfer = useTransfersStore((s) => s.updateTransfer);
  const addEntry = useHistoryStore((s) => s.addEntry);

  useEffect(() => {
    const unlisteners: Array<Promise<() => void>> = [];

    unlisteners.push(
      onPeerDiscovered((peer: PeerDiscoveredEvent) => {
        const p: Peer = {
          ...peer,
          fingerprint: peer.fingerprintShort,
          status: peer.trusted ? "paired" : "discovered",
        };
        addPeer(p);
      })
    );

    unlisteners.push(onPeerLost(({ id }) => removePeer(id)));

    unlisteners.push(
      onTransferProgress((e: TransferProgressEvent) => {
        updateTransfer(e.transferId, {
          bytesDone: e.bytesDone,
          totalBytes: e.totalBytes,
          speedBps: e.speedBps,
          etaSecs: e.etaSecs,
          status: e.status,
        });
      })
    );

    unlisteners.push(
      onTransferCompleted(({ transferId, direction }) => {
        const t = useTransfersStore.getState().transfers.find(
          (x) => x.id === transferId
        );
        if (t) {
          const peer = usePeersStore
            .getState()
            .peers.find((p) => p.id === t.peerId);
          addEntry({
            id: transferId,
            peerName: peer?.name ?? "Inconnu",
            fileName: t.files[0]?.path ?? "",
            size: t.totalBytes,
            direction,
            completedAt: new Date().toISOString(),
            status: "success",
          });
        }
      })
    );

    unlisteners.push(
      onIncomingRequest((req: IncomingRequestEvent) => {
        useIncomingStore.getState().setPending(req);
      })
    );

    return () => {
      unlisteners.forEach((p) => p.then((fn) => fn()));
    };
  }, [addPeer, removePeer, updateTransfer, addEntry]);
}
