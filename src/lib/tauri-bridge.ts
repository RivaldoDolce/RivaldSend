import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { Peer } from "../types";

// ============ COMMANDS ============

export async function startTransfer(params: {
  peerId: string;
  filePaths: string[];
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

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} o`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} Ko`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} Mo`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} Go`;
}
