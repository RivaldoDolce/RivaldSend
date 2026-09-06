import type { ReactNode } from "react";
import { useDeviceContext } from "../hooks/useDeviceContext";

export function Adaptive({
  desktop,
  mobile,
}: {
  desktop: ReactNode;
  mobile: ReactNode;
}) {
  const { isMobile } = useDeviceContext();
  return <>{isMobile ? mobile : desktop}</>;
}
