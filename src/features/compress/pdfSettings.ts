import { isInt, useStoredState } from "@/lib/storedState";
import { levelOf, limitBytes, parseShrink, type ShrinkSettings } from "./shrink";
import type { ToolOptions } from "@/features/tool/taskApi";

/** "Resolusi gambar maksimum" choices; 0 leaves images as they are. */
export const MAX_DPIS = [0, 300, 200, 150, 100];

export interface PdfSettings extends ShrinkSettings {
  /** "Kualitas gambar (1–100)"; null lets Rantakit choose. */
  quality: number | null;
  maxDpi: number;
}

function parse(stored: unknown): PdfSettings {
  const s = (stored ?? {}) as Partial<Record<keyof PdfSettings, unknown>>;
  return {
    ...parseShrink(stored),
    quality: isInt(s.quality, 1, 100) ? s.quality : null,
    maxDpi: MAX_DPIS.includes(s.maxDpi as number) ? (s.maxDpi as number) : 0,
  };
}

/** Kompres PDF's options, remembered for the next visit. */
export function usePdfSettings() {
  return useStoredState("rantakit.kompresPdf", parse);
}

export function toPdfOptions(settings: PdfSettings): ToolOptions {
  return {
    tool: "kompres-pdf",
    limit: limitBytes(settings),
    level: levelOf(settings),
    quality: settings.quality ?? undefined,
    maxDpi: settings.maxDpi || undefined,
  };
}
