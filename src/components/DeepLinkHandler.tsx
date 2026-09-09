import { useEffect } from "react";
import { usePeersStore } from "../stores/usePeersStore";
import { connectByIp } from "../lib/tauri-bridge";

function analyserLien(url: string): { ip: string; port: number; code: string } | null {
  if (!url.startsWith("rivaldsend://")) return null;
  const sansSchema = url.slice("rivaldsend://".length);
  const [autorite = "", requete = ""] = sansSchema.split("?");
  const [ip = "", portTexte] = autorite.split(":");
  const params = new URLSearchParams(requete);
  const code = (params.get("code") ?? "").trim().replace("-", "").toUpperCase();
  const port = Number(portTexte) || 53317;
  if (!ip || !/^[A-HJ-NP-Z2-9]{6}$/.test(code)) return null;
  return { ip, port, code };
}

export function DeepLinkHandler() {
  useEffect(() => {
    const gestionnaire = async (e: Event) => {
      const url = (e as CustomEvent).detail as string;
      const analyse = analyserLien(url);
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
