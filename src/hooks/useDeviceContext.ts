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
  const coarse = window.matchMedia("(pointer: coarse)").matches;
  const isTouch = coarse || navigator.maxTouchPoints > 0;
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
      isMobile: s.uaMobile || (s.isTouch && s.isNarrow),
      lowEnd: navigator.hardwareConcurrency <= 4,
      ...s,
    };
  });

  useEffect(() => {
    let alive = true;

    import("@tauri-apps/plugin-os")
      .then(({ platform }) => {
        if (!alive) return;
        const os = platform();
        const isMobileOS = os === "android" || os === "ios";
        setCtx((c) => ({
          ...c,
          os,
          isMobileOS,
          isMobile: isMobileOS || (c.isTouch && c.isNarrow),
        }));
      })
      .catch(() => {});

    let t: ReturnType<typeof setTimeout>;
    const onResize = () => {
      clearTimeout(t);
      t = setTimeout(() => {
        const s = sniff();
        setCtx((c) => ({
          ...c,
          ...s,
          isMobile: c.isMobileOS || (s.isTouch && s.isNarrow),
        }));
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
