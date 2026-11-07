import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/** Pages drawn at once; the rest wait their turn, newest asked first. */
const AT_ONCE = 2;

interface Job {
  path: string;
  page: number;
  size: number;
  resolve: (url: string | null) => void;
}

const drawn = new Map<string, Promise<string | null>>();
const waiting: Job[] = [];
let running = 0;

function next() {
  while (running < AT_ONCE && waiting.length > 0) {
    // The page asked for last is the one just scrolled into view.
    const job = waiting.pop()!;
    running += 1;
    invoke<ArrayBuffer>("page_thumbnail", { path: job.path, page: job.page, size: job.size })
      .then((bytes) =>
        bytes && bytes.byteLength > 0 ? URL.createObjectURL(new Blob([bytes], { type: "image/jpeg" })) : null,
      )
      .catch(() => null)
      .then((url) => {
        running -= 1;
        job.resolve(url);
        next();
      });
  }
}

/**
 * A blob URL of page `page` (from 0) of the PDF at `path`, `size` px on its
 * longest side. Drawn once per session; null when it can't be drawn.
 */
export function loadPage(path: string, page: number, size: number): Promise<string | null> {
  const key = `${size}:${page}:${path}`;
  let url = drawn.get(key);
  if (!url) {
    url = new Promise((resolve) => {
      waiting.push({ path, page, size, resolve });
      next();
    });
    drawn.set(key, url);
  }
  return url;
}

/**
 * The page's thumbnail once `visible` is true; null until then, while it is
 * drawn, and when it can't be. Pages out of sight are never drawn, so a
 * 200-page PDF opens at once.
 */
export function usePage(path: string | null, page: number, size: number, visible = true): string | null {
  const [loaded, setLoaded] = useState<{ key: string; url: string | null } | null>(null);
  const key = path ? `${size}:${page}:${path}` : null;
  useEffect(() => {
    if (!path || !visible || !key) return;
    let current = true;
    loadPage(path, page, size).then((url) => current && setLoaded({ key, url }));
    return () => {
      current = false;
    };
  }, [path, page, size, visible, key]);
  return loaded && loaded.key === key ? loaded.url : null;
}
