import type { ComponentType, ReactNode } from "react";
import type { TaskResult, ToolOptions } from "@/features/compress/taskApi";
import type { Translator } from "@/i18n";
import type { FileInfo } from "./fileInfo";

export interface OptionsProps<S> {
  settings: S;
  onChange: (patch: Partial<S>) => void;
  /** What is known about the chosen files, in order; files not read yet are left out. */
  files: FileInfo[];
  /** Every chosen file, in order, read or not. */
  paths: string[];
}

/** What makes one shipped Tool different on its page; the layout is the same for all. */
export interface ToolDef<S extends object> {
  /** How the Tool names its files: "3 foto", "Pilih foto"; "2 PDF", "Pilih PDF". */
  noun: "photo" | "pdf";
  /** Makes one Result from several files, in an order the user sets (X2): Gabung PDF, JPG ke PDF, Scan Dokumen. */
  ordered?: boolean;
  /** False for a Tool with nothing worth saving as a Preset, such as Gabung PDF. */
  presets?: false;
  pick: () => Promise<string[]>;
  /** The Tool's options, remembered between visits. */
  useSettings: () => readonly [S, (patch: Partial<S>) => void];
  /** Options always shown. */
  Main: ComponentType<OptionsProps<S>>;
  /** Options under "Pilihan lanjutan". */
  Advanced?: ComponentType<OptionsProps<S>>;
  /** Whether "Pilihan lanjutan" applies to the current choice; always, when left out. */
  showAdvanced?: (settings: S, files: FileInfo[]) => boolean;
  toRequest: (settings: S, files: FileInfo[]) => ToolOptions;
  /**
   * Whether `options` would leave this file as it is, like a JPG for "Ubah ke
   * JPG". Its row says so before the Task starts.
   */
  leavesUnchanged?: (options: ToolOptions, file: FileInfo) => boolean;
  /**
   * Why the main button can't be pressed yet, such as "Pilih minimal 2 PDF
   * untuk digabung", shown under it; null when it can. `count` is how many
   * files are chosen, read or not.
   */
  blocked?: (settings: S, files: FileInfo[], count: number, tr: Translator) => string | null;
  /** A note on a file's row before the Task starts, such as pages a PDF doesn't have. */
  fileNote?: (options: ToolOptions, file: FileInfo, tr: Translator) => string | null;
  /**
   * What happened, in the words that matter for this Tool (X4): "5 PDF → 1
   * PDF, 23 halaman", or "Lokasi dan info kamera dihapus dari 3 foto". Left
   * out by the compress Tools, whose Result shows each size before and after.
   */
  summary?: (results: TaskResult[], tr: Translator) => string;
  /**
   * The Tool's own work area, which takes the place of Files and Options once
   * a file is chosen, such as Atur Halaman PDF's page grid. Such a Tool works
   * on one file per Task.
   */
  Workspace?: ComponentType<WorkspaceProps>;
}

export interface WorkspaceProps {
  path: string;
  /** Pages of the PDF, once read. */
  pages: number | null;
  /** Back, the Tool's icon and its name, for the top bar. */
  heading: ReactNode;
  /** While the Task runs: its progress, in the main button's place. */
  working: ReactNode | null;
  onStart: (options: ToolOptions) => void;
  /** Whether there are changes that leaving or choosing another file would lose. */
  onUnsavedChange: (unsaved: boolean) => void;
}

export function defineTool<S extends object>(def: ToolDef<S>) {
  return def;
}
