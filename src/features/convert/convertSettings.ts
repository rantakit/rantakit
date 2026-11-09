import type { ToolOptions } from "@/features/tool/taskApi";
import type { FileInfo } from "@/features/tool/fileInfo";
import { isInt, useStoredState } from "@/lib/storedState";

export type PhotoFormat = "jpeg" | "png";
export type Background = "white" | "black" | "custom";

/** How photos become JPG; Ubah ke PNG has nothing to choose. */
export interface ConvertSettings {
  /** "Kualitas JPG (1–100)". */
  quality: number;
  /** "Warna latar" behind transparent parts when a PNG becomes a JPG. */
  background: Background;
  /** The "Pilih warna" color, "#rrggbb". */
  customColor: string;
}

const HEX = /^#[0-9a-f]{6}$/i;
const BACKGROUNDS: Record<Exclude<Background, "custom">, string> = { white: "#ffffff", black: "#000000" };

function parse(stored: unknown): ConvertSettings {
  const s = (stored ?? {}) as Partial<Record<keyof ConvertSettings, unknown>>;
  return {
    quality: isInt(s.quality, 1, 100) ? s.quality : 90,
    background: s.background === "black" || s.background === "custom" ? s.background : "white",
    customColor: typeof s.customColor === "string" && HEX.test(s.customColor) ? s.customColor : "#f2f2f2",
  };
}

/** The photo options of Ubah ke JPG, remembered for the next visit; kept where Ubah Format Foto kept them. */
export function useConvertSettings() {
  return useStoredState("rantakit.ubahFormatFoto", parse);
}

export function toConvertOptions(to: PhotoFormat, settings: ConvertSettings): ToolOptions {
  return {
    tool: "ubah-format-foto",
    to,
    quality: settings.quality,
    background:
      to === "jpeg"
        ? settings.background === "custom"
          ? settings.customColor
          : BACKGROUNDS[settings.background]
        : undefined,
  };
}

/** A file already in the chosen format is skipped. */
export function leavesUnchanged(options: ToolOptions, file: FileInfo) {
  return options.tool === "ubah-format-foto" && file.kind === (options.to === "jpeg" ? "jpg" : "png");
}
