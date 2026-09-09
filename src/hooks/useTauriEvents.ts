import { useEffect } from "react";
import {
  onPeerDiscovered,
  onPeerLost,
  onTransferProgress,
  onTransferCompleted,
  onIncomingRequest,
  onServerReady,
  notifyTransferComplete,
  formatBytes,
  connectByIp,
  type TransferProgressEvent,
  type PeerDiscoveredEvent,
  type IncomingRequestEvent,
} from "../lib/tauri-bridge";
import { useServerStore } from "../stores/useServerStore";
import { usePeersStore } from "../stores/usePeersStore";
import { useTransfersStore } from "../stores/useTransfersStore";
import { pushProgress } from "../stores/useProgressStore";
import { useHistoryStore } from "../stores/useHistoryStore";
import { useIncomingStore } from "../stores/useIncomingStore";
import { useSettingsStore } from "../stores/useSettingsStore";
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
          platform: (peer.platform as Peer["platform"]) ?? "unknown",
        };
        addPeer(p);
      })
    );

    unlisteners.push(onPeerLost(({ id }) => removePeer(id)));

    unlisteners.push(
      onTransferProgress((e: TransferProgressEvent) => {
        pushProgress(e.transferId, {
          bytesDone: e.bytesDone,
          speedBps: e.speedBps,
          etaSecs: e.etaSecs,
        });

        if (e.status !== "running") {
          updateTransfer(e.transferId, {
            status: e.status,
            totalBytes: e.totalBytes,
            error: e.error,
          });
        }
      })
    );

    unlisteners.push(
      onTransferCompleted(async ({ transferId, direction }) => {
        const t = useTransfersStore.getState().transfers.find(
          (x) => x.id === transferId
        );
        if (!t) return;

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

        if (useSettingsStore.getState().notifications) {
          await notifyTransferComplete({
            title:
              direction === "received" ? "Fichier recu" : "Envoi termine",
            body: `${t.files[0]?.path ?? "Fichier"} - ${formatBytes(t.totalBytes)}`,
          });
        }
      })
    );

    unlisteners.push(
      onIncomingRequest((req: IncomingRequestEvent) => {
        useIncomingStore.getState().setPending(req);
      })
    );

    unlisteners.push(
      onServerReady((info) => {
        useServerStore.getState().setReady({
          tls: info.tls,
          empreinte: info.empreinte,
          port: info.port,
        });
      })
    );

    return () => {
      unlisteners.forEach((p) => p.then((fn) => fn()));
    };
  }, [addPeer, removePeer, updateTransfer, addEntry]);

  useEffect(() => {
    const retryTrustedPeers = async () => {
      const trustedPeers = usePeersStore.getState().peers.filter((peer) => peer.trusted);
      await Promise.all(
        trustedPeers.map(async (peer) => {
          try {
            const discovered = await connectByIp(peer.ip, peer.port);
            addPeer({
              ...discovered,
              fingerprint: discovered.fingerprintShort,
              status: discovered.trusted ? "paired" : "discovered",
              platform: (discovered.platform as Peer["platform"]) ?? peer.platform,
            });
          } catch {
            // A peer can be offline; the next interval retries it.
          }
        }),
      );
    };

    // Reconnexion douce : toutes les 30 s suffisent, les pairs sont
    // déjà suivis en temps réel par le listener mDNS persistant.
    const timer = window.setInterval(retryTrustedPeers, 30_000);
    return () => window.clearInterval(timer);
  }, [addPeer]);
}
