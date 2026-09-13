import { useEffect } from "react";
import { usePeersStore } from "../stores/usePeersStore";
import { connectByIp, parsePairingLink } from "../lib/tauri-bridge";

export function DeepLinkHandler() {
  useEffect(() => {
    const gestionnaire = async (e: Event) => {
      const url = (e as CustomEvent).detail as string;
      const analyse = parsePairingLink(url);
      if (!analyse) return;
      try {
        const pair = await connectByIp(analyse.ip, analyse.port);
        usePeersStore.getState().addPeer({
          id: pair.id,
          name: pair.name,
          ip: pair.ip,
          port: pair.port,
          fingerprint: pair.fingerprintShort,
          fingerprintShort: pair.fingerprintShort,
          status: "discovered",
          platform: pair.platform as never,
          trusted: false,
        });
        usePeersStore.getState().setPairingCode(pair.id, analyse.code);
      } catch (err) {
        console.error("[lien] appairage impossible :", err);
      }
    };
    window.addEventListener("rivaldsend-deep-link", gestionnaire);
    return () => window.removeEventListener("rivaldsend-deep-link", gestionnaire);
  }, []);
  return null;
}
