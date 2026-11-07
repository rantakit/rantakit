import { Channel, invoke } from "@tauri-apps/api/core";
import { isAndroidApp } from "@/lib/platform";

/** Mirrors `OptionsRequest` in `src-tauri/src/commands/task.rs`. */
export type ToolOptions =
  | {
      tool: "kompres-foto";
      /** Output format; omit to keep the Original's format. */
      format?: "jpeg" | "png";
      /** The Size limit in bytes; omit to compress by `level`. */
      limit?: number;
      /** "Kecilkan saja"; defaults to "balanced" on the Rust side. */
      level?: "smallest" | "balanced" | "clearest";
      /** Exact quality 1–100 from Advanced options; wins over the limit. */
      quality?: number;
      maxDimension?: number;
    }
  | {
      tool: "kompres-pdf";
      /** The Size limit in bytes; omit to compress by `level`. */
      limit?: number;
      /** "Kecilkan saja"; defaults to "balanced" on the Rust side. */
      level?: "smallest" | "balanced" | "clearest";
      /** Exact image quality 1–100 from Advanced options; wins over the limit. */
      quality?: number;
      /** "Resolusi gambar maksimum", in dpi on an A4 page. */
      maxDpi?: number;
    }
  | {
      tool: "ubah-format-foto";
      to: "jpeg" | "png";
      /** JPG quality 1–100; defaults to 90 on the Rust side. */
      quality?: number;
      /** "#rrggbb" behind transparent parts when a PNG becomes a JPG; white when omitted. */
      background?: string;
    }
  | {
      tool: "ubah-ukuran-foto";
      /** "Ukuran baru"; defaults to "medium" on the Rust side. */
      size?: "large" | "medium" | "small";
      /** "Sisi terpanjang (px)"; wins over `size`. */
      maxSide?: number;
    }
  | {
      tool: "gabung-pdf";
      /** The user's today, "2026-11-07", for `merged 2026-11-07.pdf`. */
      date: string;
    }
  | {
      tool: "pisah-pdf";
      /** "Ambil halaman": pages counted from 1, in order. */
      pages?: number[];
      /** "Pisah semua halaman" (1) or "Setiap N halaman"; used when `pages` is left out. */
      every?: number;
    }
  | {
      tool: "atur-halaman-pdf";
      /** The Result's pages in order: a page of the Original from 0, and quarter turns clockwise. */
      pages: { page: number; turns: number }[];
    }
  | {
      tool: "jpg-ke-pdf";
      /** The user's today, for `scan 2026-11-07.pdf`. */
      date: string;
      /** "Ukuran kertas"; "photo" is "Ikuti ukuran foto". */
      paper: "a4" | "f4" | "photo";
      /** False for "Tanpa margin". */
      margin: boolean;
      /** One per photo, in order: the paper's corners (fractions, clockwise from the top left) and quarter turns. */
      edits: { corners: [number, number][] | null; turns: number }[];
    }
  | {
      tool: "scan-dokumen";
      date: string;
      paper: "a4" | "f4" | "photo";
      margin: boolean;
      edits: { corners: [number, number][] | null; turns: number }[];
      /** "Tampilan": "Bersihkan" or "Warna asli". */
      look: "clean" | "original";
    };

/** The Size limit in bytes these options aim for, if any. */
export function limitOf(options: ToolOptions): number | undefined {
  return "limit" in options ? options.limit : undefined;
}

/** Mirrors `TaskRequest` in `src-tauri/src/commands/task.rs`. */
export interface TaskRequest {
  options: ToolOptions;
  paths: string[];
}

/** Mirrors `ResultMessage` in `src-tauri/src/commands/task.rs`. */
export interface TaskResult {
  inputPath: string;
  /** The Result, or the new folder holding several (X3), such as `ijazah (pages)`. */
  outputPath: string;
  originalBytes: number;
  compressedBytes: number;
  /** When one Original made several Results: each, in order, inside `outputPath`. */
  files?: ResultPart[];
  /** Pages in a PDF made of pages: merged, taken out, or arranged. */
  pages?: number;
  /** Pages asked for that the Original doesn't have, counted from 1. */
  skippedPages?: number[];
}

/** Mirrors `PartMessage`: one of several Results from one Original. */
export interface ResultPart {
  path: string;
  bytes: number;
}

/** Every file a Result made: itself, or each file in its folder. */
export function resultFiles(result: TaskResult): ResultPart[] {
  return result.files ?? [{ path: result.outputPath, bytes: result.compressedBytes }];
}

/** Mirrors `ErrorMessage`: an error kind as a code, turned into words by `errorMessage`. */
export interface TaskError {
  code: string;
  /** Smallest size reached, for "size-limit-unreachable". */
  best?: number;
}

/** Mirrors `TaskMessage`. `file` is an index into the request's `paths`. */
export type TaskMessage =
  | { event: "progress"; file: number; fraction: number }
  | {
      event: "fileDone";
      file: number;
      outcome: { status: "done"; result: TaskResult } | { status: "failed"; error: TaskError };
    }
  | { event: "finished" };

/** Starts a Task and resolves with its id for `cancelTask`. Messages keep arriving on `onMessage` until "finished". */
export function startTask(request: TaskRequest, onMessage: (message: TaskMessage) => void) {
  const onEvent = new Channel<TaskMessage>();
  onEvent.onmessage = onMessage;
  return invoke<number>("start_task", { request, onEvent });
}

/** Finished Results stay; the file being worked on leaves nothing behind. */
export function cancelTask(id: number) {
  return invoke<void>("cancel_task", { id });
}

/** Whether Results can be saved now. Only Android 8 and 9 need the storage permission first. */
export function canSaveResults() {
  return isAndroidApp ? invoke<boolean>("can_save_results").catch(() => false) : Promise.resolve(true);
}

/** Shows Android's storage permission prompt; resolves with whether it was allowed. */
export function askToSaveResults() {
  return invoke<boolean>("ask_to_save_results").catch(() => false);
}

/** Whether a Task's progress notification may show: Android 13 and later ask once, at the first Task. */
export function notificationPermission(): Promise<"granted" | "ask" | "refused"> {
  return isAndroidApp
    ? invoke<"granted" | "ask" | "refused">("notification_permission").catch(() => "granted" as const)
    : Promise.resolve("granted");
}

/** Shows Android's notification prompt. The Task runs whatever the answer; only its notification is hidden. */
export function askNotificationPermission() {
  return invoke<boolean>("ask_notification_permission").catch(() => false);
}
