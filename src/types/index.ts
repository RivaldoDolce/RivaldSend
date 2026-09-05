export type PeerStatus = "discovered" | "paired" | "unknown";

export interface Peer {
  id: string;
  name: string;
  fingerprint: string;
  fingerprintShort: string;
  ip: string;
  port: number;
  status: PeerStatus;
  trusted: boolean;
}

export interface TransferFile {
  path: string;
  size: number;
  blake3: string;
}

export interface Transfer {
  id: string;
  files: TransferFile[];
  totalBytes: number;
  bytesDone: number;
  speedBps: number;
  etaSecs: number;
  status: "queued" | "running" | "paused" | "completed" | "failed" | "cancelled";
  peerId: string;
  createdAt: string;
  error?: string;
}

export interface HistoryEntry {
  id: string;
  peerName: string;
  fileName: string;
  size: number;
  direction: "sent" | "received";
  completedAt: string;
  status: "success" | "failed";
}
