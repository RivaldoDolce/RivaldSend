import { useEffect } from "react";
import {
  onPeerDiscovered,
  onPeerLost,
  onPeerPaired,
  onPeerForgotten,
  onTransferProgress,
  onTransferCompleted,
  onIncomingRequest,
  onServerReady,
  notifyTransferComplete,
  formatBytes,
  connectByIp,
  listTrustedPeers,
  type TransferProgressEvent,
  type PeerDiscoveredEvent,
  type IncomingRequestEvent,
  type PairedPeerEvent,
} from "../lib/tauri-bridge";
import { useServerStore } from "../stores/useServerStore";
import { usePeersStore } from "../stores/usePeersStore";
import { useTransfersStore } from "../stores/useTransfersStore";
import { pushProgress } from "../stores/useProgressStore";
import { useHistoryStore } from "../stores/useHistoryStore";
import { useIncomingStore } from "../stores/useIncomingStore";
import { useSettingsStore } from "../stores/useSettingsStore";
import type { Peer } from "../types";

function versPair(evenement: PeerDiscoveredEvent): Peer {
  return {
    ...evenement,
    fingerprint: evenement.fingerprintShort,
    status: evenement.trusted ? "paired" : "discovered",
    platform: (evenement.platform as Peer["platform"]) ?? "unknown",
  };
}

export function useTauriEvents() {
  const addPeer = usePeersStore((s) => s.addPeer);
  const removePeer = usePeersStore((s) => s.removePeer);
  const updateTransfer = useTransfersStore((s) => s.updateTransfer);
  const addEntry = useHistoryStore((s) => s.addEntry);

  useEffect(() => {
    const unlisteners: Array<Promise<() => void>> = [];

    unlisteners.push(onPeerDiscovered((peer: PeerDiscoveredEvent) => addPeer(versPair(peer))));

    unlisteners.push(onPeerLost(({ id }) => removePeer(id)));

    // Appairage entrant ou sortant : le pair rejoint la liste comme apparie.
    // C'est ce qui manquait cote PC : le mobile s'appairait, mais le PC ne
    // l'apprenait jamais.
    unlisteners.push(
      onPeerPaired((e: PairedPeerEvent) => {
        addPeer({
          id: e.peerId,
          name: e.name,
          ip: e.ip,
          port: e.port,
          fingerprint: e.fingerprintShort,
          fingerprintShort: e.fingerprintShort,
          status: "paired",
          platform: (e.platform as Peer["platform"]) ?? "unknown",
          trusted: true,
        });
      })
    );

    unlisteners.push(onPeerForgotten(({ peerId }) => removePeer(peerId)));

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
      unlisteners.forEach((p) => p.then((fn) => fn()).catch(() => {}));
    };
  }, [addPeer, removePeer, updateTransfer, addEntry]);

  // Hydratation au demarrage : les pairs apparies (magasin persiste cote
  // Rust) sont affiches immediatement, sans attendre une annonce mDNS.
  useEffect(() => {
    let annule = false;
    const hydrater = async () => {
      try {
        const apparies = await listTrustedPeers();
        if (annule) return;
        for (const p of apparies) {
          if (!p.ip || !p.port) continue;
          addPeer({
            id: p.id,
            name: p.name,
            ip: p.ip,
            port: p.port,
            fingerprint: p.fingerprintShort,
            fingerprintShort: p.fingerprintShort,
            status: "paired",
            platform: (p.platform as Peer["platform"]) ?? "unknown",
            trusted: true,
          });
        }
      } catch {
        // Hors Tauri (previsualisation web) : rien a hydrater.
      }
    };
    void hydrater();
    return () => {
      annule = true;
    };
  }, [addPeer]);

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
