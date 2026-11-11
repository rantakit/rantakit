import { useEffect, useSyncExternalStore } from "react";
import { invoke } from "@tauri-apps/api/core";
import { usePhotoEdits, type PhotoEdit } from "@/features/photo-pdf/photoEdits";

/**
 * Scan Dokumen's pages, made ahead of time (see `prepare_page` in
 * `src-tauri/src/commands/photos.rs`): each photo is straightened and cleaned
 * in the background, one at a time, as soon as its paper is found, so "Simpan
 * PDF" only packs pages that are done. A crop or turn makes that page again;
 * switching "Tampilan" makes them all again.
 */

/** The carousel's picture of a page, in px on its longest side. */
const PREVIEW_SIDE = 1200;
/** Versions kept of one photo, as the backend keeps them: both "Tampilan" choices. */
const KEPT_PER_PHOTO = 2;

export type PageState =
  /** Still looking for the paper. */
  | { status: "finding" }
  /** Waiting its turn, or being made. */
  | { status: "preparing" }
  /** Done; `url` is null when it couldn't be made, and it is made again while the PDF is saved. */
  | { status: "ready"; url: string | null };

interface Job {
  key: string;
  path: string;
  edit: PhotoEdit;
}

/** Pages made, by photo and edit: a blob URL of the page, or null. */
const made = new Map<string, string | null>();
/** Each photo's versions in `made`, oldest first. */
const versions = new Map<string, string[]>();
let queue: Job[] = [];
let running: string | null = null;

let version = 0;
const listeners = new Set<() => void>();
const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};
function changed() {
  version += 1;
  listeners.forEach((listener) => listener());
}

const keyOf = (path: string, edit: PhotoEdit) => JSON.stringify([path, edit.corners, edit.turns, edit.clean === true]);

function keep(path: string, key: string) {
  const kept = [...(versions.get(path) ?? []).filter((other) => other !== key), key];
  for (const old of kept.splice(0, Math.max(0, kept.length - KEPT_PER_PHOTO))) {
    const url = made.get(old);
    if (url) URL.revokeObjectURL(url);
    made.delete(old);
  }
  versions.set(path, kept);
}

/** Makes the next page in the queue, one at a time. */
function next() {
  if (running) return;
  const job = queue.shift();
  if (!job) return;
  running = job.key;
  changed();
  invoke<ArrayBuffer>("prepare_page", { path: job.path, edit: job.edit, size: PREVIEW_SIDE })
    .then((bytes) =>
      bytes && bytes.byteLength > 0 ? URL.createObjectURL(new Blob([bytes], { type: "image/jpeg" })) : null,
    )
    .catch(() => null)
    .then((url) => {
      made.set(job.key, url);
      keep(job.path, job.key);
      running = null;
      changed();
      next();
    });
}

/** The pages wanted now, in the order to make them; anything else waiting is dropped. */
function want(jobs: Job[]) {
  queue = jobs.filter((job) => !made.has(job.key) && job.key !== running);
  next();
}

/** Forgets the pages of photos that have left the Task. */
export function forgetPages(paths: string[]) {
  for (const path of paths) {
    for (const key of versions.get(path) ?? []) {
      const url = made.get(key);
      if (url) URL.revokeObjectURL(url);
      made.delete(key);
    }
    versions.delete(path);
  }
  queue = queue.filter((job) => !paths.includes(job.path));
  changed();
}

/**
 * Each page of `paths` as it goes in the PDF, made in the background: the
 * page at `current` first, then the ones after it, then the ones before.
 */
export function usePreparedPages(paths: string[], clean: boolean, current: number): Record<string, PageState> {
  const edits = usePhotoEdits(paths);
  useSyncExternalStore(subscribe, () => version);

  const order = [...paths.slice(current), ...paths.slice(0, current).reverse()];
  const jobs = order.flatMap((path) => {
    const state = edits[path];
    if (state.source === "finding") return [];
    const edit = { ...state.edit, clean };
    return [{ key: keyOf(path, edit), path, edit }];
  });
  const wanted = jobs.map((job) => job.key).join("\n");
  useEffect(() => want(jobs), [wanted]); // eslint-disable-line react-hooks/exhaustive-deps

  const byPath = new Map(jobs.map((job) => [job.path, job.key]));
  return Object.fromEntries(
    paths.map((path): [string, PageState] => {
      const key = byPath.get(path);
      if (!key) return [path, { status: "finding" }];
      return [path, made.has(key) ? { status: "ready", url: made.get(key) ?? null } : { status: "preparing" }];
    }),
  );
}
