import { useEffect } from "react";

export function DeepLinkHandler() {
  useEffect(() => {
    const handler = (e: Event) => {
      const url = (e as CustomEvent).detail as string;
      if (url.startsWith("rivaldsend://")) {
        console.log("Deep link reçu:", url);
      }
    };
    window.addEventListener("rivaldsend-deep-link", handler);
    return () => window.removeEventListener("rivaldsend-deep-link", handler);
  }, []);
  return null;
}
