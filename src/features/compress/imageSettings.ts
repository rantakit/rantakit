import { isInt, useStoredState } from "@/lib/storedState";
import { levelOf, limitBytes, parseShrink, type ShrinkSettings } from "./shrink";
import type { ToolOptions } from "@/features/tool/taskApi";

export type OutputFormat = "same" | "jpeg" | "png";

/**
 * Plain sizes for a photo's longest side, shared by Kompres Foto and Ubah
 * Ukuran Foto. Mirrors `PhotoSize` in crates/rantakit-tasks/src/task.rs.
 */
export type PhotoSize = "large" | "medium" | "small";
export const PHOTO_SIZES: Record<PhotoSize, number> = { large: 1920, medium: 1280, small: 800 };

export interface ImageSettings extends ShrinkSettings {
  format: OutputFormat;
  /** "Ukuran gambar"; "original" leaves it to the choice above. */
  size: PhotoSize | "original";
  /** "Kualitas (1–100)"; null lets Rantakit choose. */
  quality: number | null;
}

/** Falls back to the defaults for anything missing or invalid. */
function parse(stored: unknown): ImageSettings {
  const s = (stored ?? {}) as Partial<Record<keyof ImageSettings, unknown>>;
  return {
    ...parseShrink(stored),
    format: s.format === "jpeg" || s.format === "png" ? s.format : "same",
    size: typeof s.size === "string" && s.size in PHOTO_SIZES ? (s.size as PhotoSize) : "original",
    quality: isInt(s.quality, 1, 100) ? s.quality : null,
  };
}

/** Kompres Foto's options, remembered for the next visit. */
export function useImageSettings() {
  return useStoredState("rantakit.kompresFoto", parse);
}

export function toImageOptions(settings: ImageSettings): ToolOptions {
  return {
    tool: "kompres-foto",
    format: settings.format === "same" ? undefined : settings.format,
    limit: limitBytes(settings),
    level: levelOf(settings),
    quality: settings.quality ?? undefined,
    maxDimension: settings.size === "original" ? undefined : PHOTO_SIZES[settings.size],
  };
}

/** Longest sides tried, in order, when a Size limit can't be reached. */
const SHRINK_STEPS = [1920, 1280, 960, 640];

/**
 * The follow-up to "Paling kecil bisa …": the same options with smaller
 * images. Null when the options have no Size limit or can't shrink further.
 */
export function withSmallerImages(options: ToolOptions): ToolOptions | null {
  if (options.tool !== "kompres-foto" || options.limit === undefined) return null;
  const current = options.maxDimension ?? Infinity;
  const next = SHRINK_STEPS.find((side) => side < current);
  return next === undefined ? null : { ...options, maxDimension: next };
}
