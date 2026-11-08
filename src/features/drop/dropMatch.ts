import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ToolId } from "@/features/home/tools";

/** What Rantakit can do with dragged files (`FileMatch` in rantakit-tasks). */
export type DropMatch =
  | { status: "fits"; kinds: string[]; tools: ToolId[] }
  | { status: "unsupported" }
  | { status: "mixed" }
  /** An older `.doc`, `.xls`, or `.ppt`. */
  | { status: "oldOffice" };

const PHOTO_KINDS = ["jpg", "png", "heic", "webp"];
const DOCUMENT_KINDS = ["word", "excel", "powerpoint", "locked-office"];

/** How to count the files: "3 foto", "3 PDF", "3 dokumen", or "3 file". */
export function filesNoun(match: DropMatch): "photo" | "pdf" | "document" | "any" {
  if (match.status !== "fits") return "any";
  if (match.kinds.every((kind) => PHOTO_KINDS.includes(kind))) return "photo";
  if (match.kinds.every((kind) => kind === "pdf")) return "pdf";
  if (match.kinds.every((kind) => DOCUMENT_KINDS.includes(kind))) return "document";
  return "any";
}

export function matchFiles(paths: string[]): Promise<DropMatch> {
  return invoke<DropMatch>("match_files", { paths }).catch(() => ({ status: "unsupported" }) as const);
}

/** The match for the files being dragged; null while it is worked out. */
export function useDropMatch(paths: string[] | null): DropMatch | null {
  const [result, setResult] = useState<{ paths: string[]; match: DropMatch } | null>(null);
  useEffect(() => {
    if (!paths) return;
    let current = true;
    matchFiles(paths).then((match) => {
      if (current) setResult({ paths, match });
    });
    return () => {
      current = false;
    };
  }, [paths]);
  return result && result.paths === paths ? result.match : null;
}
