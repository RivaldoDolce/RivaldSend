import { useEffect, useState } from "react";

export interface DeviceContext {
  os: string;
  isMobileOS: boolean;
  isTouch: boolean;
  isNarrow: boolean;
  isMobile: boolean;
  lowEnd: boolean;
}

function sniff() {
  const isTouch = window.matchMedia("(pointer: coarse)").matches || navigator.maxTouchPoints > 0;
  const isNarrow = window.innerWidth < 820;
  const uaMobile = /Android|iPhone|iPad|Mobile/i.test(navigator.userAgent);
  return { isTouch, isNarrow, uaMobile };
}

export function useDeviceContext(): DeviceContext {
  const [ctx, setCtx] = useState<DeviceContext>(() => {
    const s = sniff();
    return {
      os: "unknown",
      isMobileOS: s.uaMobile,
      isTouch: s.isTouch,
      isNarrow: s.isNarrow,
      isMobile: s.uaMobile,
      lowEnd: navigator.hardwareConcurrency <= 4,
    };
  });

  useEffect(() => {
    let alive = true;
    let tauriResolved = false;

    import("@tauri-apps/plugin-os")
      .then(({ platform }) => {
        if (!alive) return;
        tauriResolved = true;
        const os = platform();
        const isMobileOS = os === "android" || os === "ios";
        setCtx((c) => ({ ...c, os, isMobileOS, isMobile: isMobileOS }));
      })
      .catch(() => {});

    let t: ReturnType<typeof setTimeout>;
    const onResize = () => {
      clearTimeout(t);
      t = setTimeout(() => {
        const s = sniff();
        setCtx((c) => {
          if (tauriResolved) {
            return { ...c, isTouch: s.isTouch, isNarrow: s.isNarrow };
          }
          const fallbackMobile = s.uaMobile || (s.isTouch && s.isNarrow);
          return { ...c, isTouch: s.isTouch, isNarrow: s.isNarrow, isMobileOS: s.uaMobile, isMobile: fallbackMobile };
        });
      }, 150);
    };
    window.addEventListener("resize", onResize);
    return () => {
      alive = false;
      clearTimeout(t);
      window.removeEventListener("resize", onResize);
    };
  }, []);

  return ctx;
}
