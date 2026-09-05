import { useEffect } from "react";
import { usePeersStore } from "../stores/usePeersStore";
import { pickFiles } from "../lib/tauri-bridge";

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return (
    tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || target.isContentEditable
  );
}

export function useKeyboardShortcuts() {
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && usePeersStore.getState().showSendModal) {
        usePeersStore.getState().closeSendModal();
        return;
      }

      if (
        (e.ctrlKey || e.metaKey) &&
        e.key.toLowerCase() === "o" &&
        !isEditableTarget(e.target)
      ) {
        e.preventDefault();
        pickFiles()
          .then((paths) => {
            if (paths && paths.length > 0) {
              usePeersStore
                .getState()
                .openSendModal(paths.map((p) => ({ path: p, size: 0 })));
            }
          })
          .catch((err) => console.error("[shortcuts] pickFiles failed:", err));
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);
}
