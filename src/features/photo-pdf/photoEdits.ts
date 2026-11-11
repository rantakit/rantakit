import { useEffect, useState, useSyncExternalStore } from "react";
import { invoke } from "@tauri-apps/api/core";

/** Four corners as fractions (0–1) of the upright photo, clockwise from the top left. */
export type Corners = [[number, number], [number, number], [number, number], [number, number]];

/** How one photo becomes a page; mirrors `EditRequest` in `src-tauri/src/commands/photos.rs`. */
export interface PhotoEdit {
  /** The paper to crop and straighten to; the whole photo when null. */
  corners: Corners | null;
  /** Quarter turns clockwise. */
  turns: number;
  /** "Bersihkan", for a Scan Dokumen row's preview; the Task takes it from "Tampilan". */
  clean?: boolean;
}

/**
 * Where a photo's edit came from, for the note on its row: still looking
 * for the paper, found it, didn't, or set by the user in the crop editor.
 */
export type EditSource = "finding" | "found" | "notFound" | "user";

export interface EditState {
  edit: PhotoEdit;
  source: EditSource;
}

export const WHOLE: Corners = [
  [0, 0],
  [1, 0],
  [1, 1],
  [0, 1],
];

const NO_EDIT: PhotoEdit = { corners: null, turns: 0 };

/** Each photo's edit, kept for the session so a photo chosen again keeps its crop. */
const states = new Map<string, EditState>();
const listeners = new Set<() => void>();
let version = 0;

function set(path: string, state: EditState) {
  states.set(path, state);
  version += 1;
  listeners.forEach((listener) => listener());
}

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};

/** Looks for the paper in a photo the first time it is seen. */
function find(path: string) {
  if (states.has(path)) return;
  set(path, { edit: NO_EDIT, source: "finding" });
  invoke<Corners | null>("find_paper", { path })
    .catch(() => null)
    .then((corners) => {
      // The user may have set it already.
      if (states.get(path)?.source !== "finding") return;
      set(path, { edit: { corners, turns: 0 }, source: corners ? "found" : "notFound" });
    });
}

/** The edit of each of `paths`, looking for the paper in photos not seen before. */
export function usePhotoEdits(paths: string[]): Record<string, EditState> {
  useSyncExternalStore(subscribe, () => version);
  useEffect(() => paths.forEach(find), [paths]);
  return Object.fromEntries(paths.map((path) => [path, states.get(path) ?? { edit: NO_EDIT, source: "finding" }]));
}

/** The user's own crop from the editor. */
export function setPhotoEdit(path: string, edit: PhotoEdit) {
  set(path, { edit: { corners: isWhole(edit.corners) ? null : edit.corners, turns: ((edit.turns % 4) + 4) % 4 }, source: "user" });
}

/** Turns a photo a quarter clockwise; its crop, and where that came from, stay. */
export function turnPhoto(path: string) {
  const state = states.get(path);
  if (!state || state.source === "finding") return;
  set(path, { ...state, edit: { ...state.edit, turns: (state.edit.turns + 1) % 4 } });
}

/** The edits for a Task, in the order of `paths`. */
export function photoEditsFor(paths: string[]): PhotoEdit[] {
  return paths.map((path) => states.get(path)?.edit ?? NO_EDIT);
}

const isWhole = (corners: Corners | null) =>
  corners === null || corners.every(([x, y], i) => Math.abs(x - WHOLE[i][0]) < 1e-3 && Math.abs(y - WHOLE[i][1]) < 1e-3);

/** Whether the photo goes on its page as it is. */
export const isUnedited = (edit: PhotoEdit) => edit.corners === null && edit.turns % 4 === 0;

const previews = new Map<string, Promise<string | null>>();

function loadPreview(path: string, edit: PhotoEdit, size: number) {
  const key = JSON.stringify([path, edit, size]);
  let url = previews.get(key);
  if (!url) {
    url = invoke<ArrayBuffer>("edited_preview", { path, edit, size })
      .then((bytes) =>
        bytes && bytes.byteLength > 0 ? URL.createObjectURL(new Blob([bytes], { type: "image/jpeg" })) : null,
      )
      .catch(() => null);
    previews.set(key, url);
  }
  return url;
}

/**
 * A blob URL of the photo as it will go on its page, `size` px on its
 * longest side; null while it loads or when there is nothing to show yet.
 */
export function useEditedPreview(path: string, edit: PhotoEdit | null, size: number): string | null {
  const key = edit ? JSON.stringify([path, edit, size]) : null;
  const [loaded, setLoaded] = useState<{ key: string; url: string | null } | null>(null);
  useEffect(() => {
    if (!edit || !key) return;
    let current = true;
    loadPreview(path, edit, size).then((url) => current && setLoaded({ key, url }));
    return () => {
      current = false;
    };
    // `key` stands for `edit`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return loaded && loaded.key === key ? loaded.url : null;
}

/** The photo open in the crop editor, if any; one at a time. */
let editing: string | null = null;
const editingListeners = new Set<() => void>();

export function openCropEditor(path: string | null) {
  editing = path;
  editingListeners.forEach((listener) => listener());
}

/** The photo open in the crop editor. */
export function useCropEditing(): string | null {
  return useSyncExternalStore(
    (listener) => {
      editingListeners.add(listener);
      return () => editingListeners.delete(listener);
    },
    () => editing,
  );
}
