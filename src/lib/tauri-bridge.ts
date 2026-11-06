import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

// ============ COMMANDS ============

export async function startTransfer(params: {
  peerId: string;
  filePaths: string[];
  code?: string;
}): Promise<{ transferId: string }> {
  return invoke("start_transfer", params);
}

export async function cancelTransfer(transferId: string): Promise<void> {
  return invoke("cancel_transfer", { transferId });
}

export async function pauseTransfer(transferId: string): Promise<void> {
  return invoke("pause_transfer", { transferId });
}

export async function resumeTransfer(transferId: string): Promise<void> {
  return invoke("resume_transfer", { transferId });
}

export async function acceptIncoming(params: {
  requestId: string;
  targetDir: string;
}): Promise<void> {
  return invoke("accept_incoming", params);
}

export async function rejectIncoming(requestId: string): Promise<void> {
  return invoke("reject_incoming", { requestId });
}

export async function getDeviceInfo(): Promise<{
  name: string;
  ip: string;
  fingerprint: string;
  fingerprintShort: string;
  port: number;
}> {
  return invoke("get_device_info");
}

export async function generatePairingQr(params: {
  ip: string;
  port: number;
  code: string;
  fingerprintShort: string;
}): Promise<string> {
  return invoke("generate_pairing_qr", params);
}

export async function listNetworkInterfaces(): Promise<
  Array<[string, string]>
> {
  return invoke("list_network_interfaces");
}

export async function checkFirewall(): Promise<string> {
  return invoke("check_firewall");
}

export async function pingPeer(ip: string, port: number): Promise<number> {
  return invoke<number>("ping_peer", { ip, port });
}

export async function connectByIp(ip: string, port: number): Promise<PeerDiscoveredEvent> {
  return invoke<PeerDiscoveredEvent>("connect_by_ip", { ip, port });
}

export async function rescanPeers(): Promise<void> {
  return invoke("rescan_peers");
}

export async function approvePeer(peerId: string, code?: string): Promise<void> {
  return invoke("approve_peer", { peerId, code: code ?? null });
}

export async function genererCodeAppairage(): Promise<string> {
  return invoke<string>("generer_code_appairage");
}

export async function setPairingCodeBackend(code: string): Promise<string> {
  return invoke<string>("set_pairing_code", { code });
}

export type PairedPeerEvent = {
  peerId: string;
  deviceId: string;
  name: string;
  ip: string;
  port: number;
  fingerprintShort: string;
  platform: string;
};

export async function pairWithPeer(params: {
  ip: string;
  port: number;
  code: string;
}): Promise<PairedPeerEvent> {
  return invoke<PairedPeerEvent>("pair_with_peer", params);
}

export type TrustedPeer = {
  id: string;
  deviceId: string;
  name: string;
  ip: string;
  port: number;
  fingerprintShort: string;
  platform: string;
  secret: string;
  pairedAt?: string | null;
};

export async function listTrustedPeers(): Promise<TrustedPeer[]> {
  return invoke<TrustedPeer[]>("list_trusted_peers");
}

export async function forgetPeer(peerId: string): Promise<boolean> {
  return invoke<boolean>("forget_peer", { peerId });
}

export async function forgetAllPeers(): Promise<number> {
  return invoke<number>("forget_all_peers");
}

export async function setDownloadDirBackend(targetDir: string): Promise<void> {
  return invoke("set_download_dir", { targetDir });
}

// ============ DIALOG HELPERS ============

export async function pickFolder(): Promise<string | null> {
  const result = await open({
    directory: true,
    multiple: false,
    title: "Choisir le dossier de réception",
  });
  return (result as string) ?? null;
}

export async function pickFiles(): Promise<string[] | null> {
  const result = await open({
    directory: false,
    multiple: true,
    title: "Choisir des fichiers",
  });
  if (Array.isArray(result)) return result as string[];
  if (result) return [result as string];
  return null;
}

export async function openFileDialog(): Promise<string[] | null> {
  return invoke<string[] | null>("open_file_dialog");
}

export type PairingLink = { ip: string; port: number; code: string };

// Analyse un lien rivaldsend://ip:port?code=XXXXXX&fp=... (QR, lien copié, deep link).
export function parsePairingLink(url: string): PairingLink | null {
  if (!url.startsWith("rivaldsend://")) return null;
  const sansSchema = url.slice("rivaldsend://".length);
  const [autorite = "", requete = ""] = sansSchema.split("?");
  const [ip = "", portTexte] = autorite.split(":");
  if (!ip) return null;
  // N'accepte qu'une adresse IP litterale : un nom d'hote ferait de l'app
  // un relais vers un tiers, et le backend refuserait de toute facon.
  if (!estAdresseIp(ip)) return null;
  const params = new URLSearchParams(requete);
  const code = (params.get("code") ?? "").trim().replace("-", "").toUpperCase();
  if (!/^[A-HJ-NP-Z2-9]{6}$/.test(code) && !/^[0-9]{6}$/.test(code)) return null;
  const port = Number(portTexte) || 53317;
  if (!Number.isInteger(port) || port < 1 || port > 65535) return null;
  return { ip, port, code };
}

// Validation d'adresse IP litterale (v4 ou v6), sans dependance externe.
// Utilisee avant tout appel reseau declenche par un QR ou un lien.
export function estAdresseIp(texte: string): boolean {
  const t = texte.trim();
  // IPv4 : 4 groupes decimaux.
  const v4 = t.match(/^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/);
  if (v4) {
    return v4.slice(1).every((g) => {
      const n = Number(g);
      return n >= 0 && n <= 255 && String(n) === g.replace(/^0+(?=\d)/, "");
    });
  }
  // IPv6 : presence de `:` et caracteres hexadecimaux uniquement.
  if (t.includes(":")) {
    // Forme `[::1]` (avec crochets) ou `::1` (sans).
    const sansCrochets = t.replace(/^\[(.*)\]$/, "$1");
    if (/^[0-9a-fA-F:]+$/.test(sansCrochets) && sansCrochets.includes(":")) {
      const parties = sansCrochets.split("::");
      if (parties.length <= 2) {
        const groupes = sansCrochets.replace(/^:|:$/g, "").split(/:+/).filter(Boolean);
        if (groupes.length <= 8 && groupes.every((g) => /^[0-9a-fA-F]{1,4}$/.test(g))) {
          return true;
        }
      }
    }
    return false;
  }
  return false;
}

// Scanne un QR avec la caméra (mobile uniquement, plugin barcode-scanner).
// Gère les permissions au runtime et retourne null si l'utilisateur annule.
export async function scanQrCode(): Promise<PairingLink | null> {
  const { scan, checkPermissions, requestPermissions } =
    await import("@tauri-apps/plugin-barcode-scanner");

  let perm: string = await checkPermissions();
  if (perm === "prompt" || perm === "prompt-with-rationale") {
    perm = await requestPermissions();
  }
  if (perm !== "granted") {
    throw new Error(
      perm === "denied"
        ? "Permission caméra refusée — activez-la dans les paramètres système."
        : `Permission caméra non accordée (${perm}).`
    );
  }

  const res = await scan({ cameraDirection: "back", windowed: false });
  if (!res?.content) return null;
  const lien = parsePairingLink(res.content.trim());
  if (!lien) {
    throw new Error("QR invalide — attendu : rivaldsend://ip:port?code=XXXXXX");
  }
  return lien;
}

// Dossier de réception sur mobile : le sélecteur de dossier natif n'existe pas
// via le plugin dialog, donc on essaie le dossier Download public puis on
// replie vers un sous-dossier privé de l'app (toujours accessible en écriture).
export async function initMobileDownloadDir(): Promise<string> {
  const { downloadDir, appDataDir } = await import("@tauri-apps/api/path");
  const essais: string[] = [];
  try {
    essais.push(await downloadDir());
  } catch {
    // Pas de dossier public accessible, on passe au repli privé.
  }
  try {
    essais.push((await appDataDir()) + "/RivaldSend");
  } catch {
    // Dernier recours géré par le backend (/tmp ou équivalent).
  }
  let dernierErreur: unknown = null;
  for (const dossier of essais) {
    try {
      await setDownloadDirBackend(dossier);
      return dossier;
    } catch (err) {
      dernierErreur = err;
    }
  }
  throw dernierErreur instanceof Error
    ? dernierErreur
    : new Error("Aucun dossier de réception accessible");
}

// ============ EVENT TYPES ============

export type TransferProgressEvent = {
  transferId: string;
  bytesDone: number;
  totalBytes: number;
  speedBps: number;
  etaSecs: number;
  status: "running" | "paused" | "completed" | "failed" | "cancelled";
  error?: string;
};

export type PeerDiscoveredEvent = {
  id: string;
  name: string;
  ip: string;
  port: number;
  fingerprintShort: string;
  trusted: boolean;
  platform: string;
};

export type IncomingRequestEvent = {
  requestId: string;
  peerId: string;
  peerName: string;
  files: Array<{ name: string; size: number }>;
  totalBytes: number;
};

// ============ EVENT LISTENERS ============

export function onTransferProgress(
  cb: (e: TransferProgressEvent) => void
): Promise<UnlistenFn> {
  return listen<TransferProgressEvent>("transfer_progress", (e) =>
    cb(e.payload)
  );
}

export function onPeerDiscovered(
  cb: (e: PeerDiscoveredEvent) => void
): Promise<UnlistenFn> {
  return listen<PeerDiscoveredEvent>("peer_discovered", (e) => cb(e.payload));
}

export function onPeerLost(cb: (e: { id: string }) => void): Promise<UnlistenFn> {
  return listen<{ id: string }>("peer_lost", (e) => cb(e.payload));
}

export function onIncomingRequest(
  cb: (e: IncomingRequestEvent) => void
): Promise<UnlistenFn> {
  return listen<IncomingRequestEvent>("incoming_request", (e) => cb(e.payload));
}

export function onTransferCompleted(
  cb: (e: { transferId: string; direction: "sent" | "received" }) => void
): Promise<UnlistenFn> {
  return listen<{ transferId: string; direction: "sent" | "received" }>(
    "transfer_completed",
    (e) => cb(e.payload)
  );
}

export function onPeerPaired(cb: (e: PairedPeerEvent) => void): Promise<UnlistenFn> {
  return listen<PairedPeerEvent>("peer_paired", (e) => cb(e.payload));
}

export function onPeerForgotten(cb: (e: { peerId: string }) => void): Promise<UnlistenFn> {
  return listen<{ peerId: string }>("peer_forgotten", (e) => cb(e.payload));
}

export type ServerReadyEvent = {
  port: number;
  tls: boolean;
  empreinte?: string;
  erreur?: string;
};

export function onServerReady(
  cb: (e: ServerReadyEvent) => void
): Promise<UnlistenFn> {
  return listen<ServerReadyEvent>("server_ready", (e) => cb(e.payload));
}

// ============ SYSTEM NOTIFICATIONS ============

export async function notifyTransferComplete(params: {
  title: string;
  body: string;
}): Promise<void> {
  try {
    const {
      isPermissionGranted,
      requestPermission,
      sendNotification,
    } = await import("@tauri-apps/plugin-notification");

    let granted = await isPermissionGranted();
    if (!granted) {
      granted = (await requestPermission()) === "granted";
    }
    if (!granted) return;

    sendNotification({ title: params.title, body: params.body });
  } catch (err) {
    console.error("[notify] system notification failed:", err);
  }
}

// ============ HELPERS ============

export { formatBytes } from "./utils";
