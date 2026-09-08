import { useMemo } from "react";
import type { Peer } from "../types";
import { useSettingsStore } from "../stores/useSettingsStore";

const PALETTE_LIGHT: [string, string][] = [
  ["#0A84FF", "#E8F2FF"],
  ["#30D158", "#E8FFE8"],
  ["#FF9F0A", "#FFF4E8"],
  ["#AF52DE", "#F4E8FF"],
  ["#FF453A", "#FFE8E8"],
  ["#5AC8FA", "#E8F8FF"],
  ["#FFD60A", "#FFFDE8"],
];
const PALETTE_DARK: [string, string][] = [
  ["#409CFF", "#1A2A44"],
  ["#30D158", "#14301E"],
  ["#FF9F0A", "#2F2410"],
  ["#C08CFF", "#2A1E3A"],
  ["#FF453A", "#331515"],
  ["#5AC8FA", "#14313E"],
  ["#FFD60A", "#332E0A"],
];

export function PeerAvatar({
  peer,
  size = 40,
}: {
  peer: Peer;
  size?: number;
}) {
  const darkMode = useSettingsStore((s) => s.darkMode);
  const { bg, fg, borderRadius } = useMemo(() => {
    const pal = darkMode ? PALETTE_DARK : PALETTE_LIGHT;
    const hash =
      peer.fingerprintShort.charCodeAt(0) ^
      (peer.fingerprintShort.charCodeAt(1) ?? 0);
    const palette = pal[hash % pal.length]!;
    const shapes = ["50%", "12px", "20%"];
    return {
      bg: palette[1],
      fg: palette[0],
      borderRadius: shapes[hash % shapes.length],
    };
  }, [peer.fingerprintShort, darkMode]);

  return (
    <div
      className="shrink-0 flex items-center justify-center font-bold"
      style={{
        width: size,
        height: size,
        backgroundColor: bg,
        color: fg,
        borderRadius,
        fontSize: size * 0.4,
      }}
    >
      {peer.name.charAt(0).toUpperCase()}
    </div>
  );
}
