import { useEffect, useState, type ComponentType } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { IconProps } from "@keyline-icons/react";
import {
  AudioLines,
  FileArrowDown,
  FileImage,
  FileText,
  FileZip,
  Film,
  Grid2x2,
  Image,
  ImageArrowDown,
  Layers,
  MapPinOff,
  Maximize2,
  MusicNote,
  ScanText,
  Split,
  Stamp,
} from "@keyline-icons/react/two-tone";

/** The ids in crates/rantakit-tasks/src/catalogue.rs (`ToolId::as_str`). */
export type ToolId =
  | "kompres-foto"
  | "kompres-pdf"
  | "ubah-ke-jpg"
  | "ubah-ke-png"
  | "ubah-ukuran-foto"
  | "kompres-dokumen"
  | "kompres-audio"
  | "ubah-format-audio"
  | "kompres-video"
  | "ubah-ke-pdf"
  | "scan-dokumen"
  | "gabung-pdf"
  | "pisah-pdf"
  | "atur-halaman-pdf"
  | "beri-watermark"
  | "hapus-info-lokasi";

/** The Jobs, in the order the All tools page shows them (`Job::as_str`). */
export const JOBS = ["kecilkan-ubah", "scan", "atur-pdf", "ambil-teks", "bagikan-aman"] as const;
export type JobId = (typeof JOBS)[number];

/** A Tool listed on this platform, in catalogue order. */
export interface ToolInfo {
  id: ToolId;
  job: JobId;
  comingSoon: boolean;
}

export const TOOL_ICONS: Record<ToolId, ComponentType<IconProps>> = {
  "kompres-foto": ImageArrowDown,
  "kompres-pdf": FileArrowDown,
  "ubah-ke-jpg": FileImage,
  "ubah-ke-png": Image,
  "ubah-ukuran-foto": Maximize2,
  "kompres-dokumen": FileZip,
  "kompres-audio": AudioLines,
  "ubah-format-audio": MusicNote,
  "kompres-video": Film,
  "ubah-ke-pdf": FileText,
  "scan-dokumen": ScanText,
  "gabung-pdf": Layers,
  "pisah-pdf": Split,
  "atur-halaman-pdf": Grid2x2,
  "beri-watermark": Stamp,
  "hapus-info-lokasi": MapPinOff,
};

/**
 * The Tools in Home's grid: the recently used ones, newest first, topped up
 * with this platform's defaults (`home_tools`). Empty until the backend answers.
 */
export function useHomeTools(): ToolId[] {
  const [tools, setTools] = useState<ToolId[]>([]);
  useEffect(() => {
    invoke<ToolId[]>("home_tools")
      .then(setTools)
      .catch(() => {});
  }, []);
  return tools;
}

/** The Tools to list on this platform; empty until the backend answers. */
export function useTools(): ToolInfo[] {
  const [tools, setTools] = useState<ToolInfo[]>([]);
  useEffect(() => {
    invoke<ToolInfo[]>("list_tools")
      .then(setTools)
      .catch(() => {});
  }, []);
  return tools;
}
