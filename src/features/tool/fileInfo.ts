import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { usePage } from "@/features/pages/pageThumbnails";

/** Mirrors `FileInfo` in `src-tauri/src/commands/files.rs`; null when it can't be read. */
export interface FileInfo {
  size: number | null;
  kind: string | null;
  pages: number | null;
  longestSide: number | null;
}

const infos = new Map<string, FileInfo>();

/** Size, page count and so on for each path, read once per path and kept for the session. */
export function useFileInfo(paths: string[]): Record<string, FileInfo> {
  const [, rerender] = useState(0);
  useEffect(() => {
    const missing = paths.filter((path) => !infos.has(path));
    if (missing.length === 0) return;
    let current = true;
    invoke<FileInfo[]>("file_info", { paths: missing })
      .then((read) => {
        missing.forEach((path, i) => read[i] && infos.set(path, read[i]));
        if (current) rerender((n) => n + 1);
      })
      .catch(() => {});
    return () => {
      current = false;
    };
  }, [paths]);
  return Object.fromEntries(paths.flatMap((path) => (infos.has(path) ? [[path, infos.get(path)!]] : [])));
}

const thumbnails = new Map<string, Promise<string | null>>();

function loadThumbnail(path: string, size: number) {
  const key = `${size}:${path}`;
  let url = thumbnails.get(key);
  if (!url) {
    url = invoke<ArrayBuffer>("thumbnail", { path, size })
      .then((bytes) =>
        bytes && bytes.byteLength > 0 ? URL.createObjectURL(new Blob([bytes], { type: "image/jpeg" })) : null,
      )
      .catch(() => null);
    thumbnails.set(key, url);
  }
  return url;
}

/**
 * The picture for a file's tile: the photo itself, or a PDF's first page
 * (X1), `size` px on its longest side; null while it loads.
 */
export function useFileTile(path: string | null, noun: "photo" | "pdf", size: number) {
  const photo = useThumbnail(noun === "photo" ? path : null, size);
  const page = usePage(noun === "pdf" ? path : null, 0, size);
  return noun === "pdf"
    ? { thumbnail: page, thumbnailKind: "page" as const }
    : { thumbnail: photo, thumbnailKind: "photo" as const };
}

/**
 * A blob URL of a small JPG of the photo at `path`, `size` px on its longest
 * side; null while it loads, for PDFs, and when it can't be made.
 */
export function useThumbnail(path: string | null, size: number): string | null {
  const [loaded, setLoaded] = useState<{ path: string; url: string | null } | null>(null);
  useEffect(() => {
    if (!path) return;
    let current = true;
    loadThumbnail(path, size).then((url) => current && setLoaded({ path, url }));
    return () => {
      current = false;
    };
  }, [path, size]);
  return loaded && loaded.path === path ? loaded.url : null;
}
