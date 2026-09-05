import { useTransfersStore } from "../stores/useTransfersStore";

export function useActiveTransfersCount(): number {
  return useTransfersStore((s) =>
    s.transfers.filter((t) => t.status === "running" || t.status === "queued")
      .length
  );
}

export function useHasActiveTransfers(): boolean {
  return useTransfersStore((s) =>
    s.transfers.some((t) => t.status === "running" || t.status === "queued")
  );
}
