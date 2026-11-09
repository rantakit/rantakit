import { useState } from "react";
import { useCompressJob } from "./useCompressJob";
import type { Refusal } from "@/features/drop/useIncomingFiles";
import { useT } from "@/i18n";
import { nounOf, pageNoun } from "./fileInfo";
import { toolDef, type ShippedTool } from "./tools";

const unique = (paths: string[]) => [...new Set(paths)];

/** What to do once the user agrees to leave the Tool page, and whether that stops a Task. */
interface Leaving {
  action: () => void;
  working: boolean;
}

/**
 * The Tool page on screen: its Tool, its Originals, and its Task, with the
 * question asked before anything on it would be lost. Render
 * `ToolSessionDialogs` with the result.
 */
export function useToolSession() {
  const { t } = useT();
  /** The Tool page on screen, over Home. */
  const [tool, setTool] = useState<ShippedTool | null>(null);
  /** The Originals added to that page. */
  const [paths, setPaths] = useState<string[]>([]);
  const job = useCompressJob();
  const [leaving, setLeaving] = useState<Leaving | null>(null);
  /** A Tool with its own work area (Atur Halaman PDF) has changes that leaving would lose. */
  const [unsaved, setUnsaved] = useState(false);
  /** Another file chosen for such a Tool, waiting for "Ganti PDF?" to be answered. */
  const [replacing, setReplacing] = useState<string | null>(null);

  /** Opens `id`'s page with these files, or none. */
  const open = (id: ShippedTool, withPaths: string[] = []) => {
    setTool(id);
    setPaths(unique(withPaths));
    job.reset();
  };

  /** Clears the Tool page; a running Task is stopped, and its finished Results stay. */
  const close = () => {
    if (tool) toolDef(tool).discard?.(paths);
    setTool(null);
    setPaths([]);
    job.reset();
  };

  /**
   * Whether leaving now loses the files chosen, before any Result. A work
   * area loses nothing until it is changed.
   */
  const wouldLoseFiles = () =>
    paths.length > 0 && job.items.length === 0 && (!tool || !toolDef(tool).Workspace || unsaved);

  /** Runs `action`, which leaves the Tool page, after asking if files would be lost or a Task stopped. */
  const guard = (action: () => void) => {
    if (tool && (wouldLoseFiles() || job.isRunning)) setLeaving({ action, working: job.isRunning });
    else action();
  };

  /** Adds Originals to the page, from the picker or a drop; says why when they can't go on it. */
  const addFiles = (added: string[]): Refusal | null => {
    if (job.isRunning || added.length === 0) return null;
    // A Tool with its own work area takes one file per Task; another one replaces it.
    if (tool && toolDef(tool).Workspace) {
      const next = added[0];
      if (next === paths[0] && job.items.length === 0) return null;
      if (unsaved && job.items.length === 0) {
        setReplacing(next);
        return null;
      }
      job.reset();
      setPaths([next]);
      return null;
    }
    // Beri Watermark takes photos or PDFs, but not both on one page (X5).
    if (tool && toolDef(tool).noun === "file") {
      const kept = job.items.length > 0 ? [] : paths;
      if (new Set([...kept, ...added].map(nounOf)).size > 1) {
        return { title: t("drop.mixed"), detail: t("drop.mixed.choose") };
      }
    }
    // After a Result, new files start over.
    if (job.items.length > 0) {
      job.reset();
      setPaths(unique(added));
    } else {
      setPaths((prev) => unique([...prev, ...added]));
    }
    return null;
  };

  /** The Tool's own picker; a Tool just opened from elsewhere has no files yet. */
  const pick = (id: ShippedTool) =>
    toolDef(id).pick(pageNoun(toolDef(id).noun, id === tool && job.items.length === 0 ? paths : []));

  const remove = (path: string) => {
    if (tool) toolDef(tool).discard?.([path]);
    setPaths((prev) => prev.filter((p) => p !== path));
  };

  /** "Proses lagi": the same Tool, empty. */
  const processAnother = () => {
    if (tool) toolDef(tool).discard?.(paths);
    job.reset();
    setPaths([]);
  };

  return {
    tool,
    paths,
    job,
    open,
    close,
    guard,
    addFiles,
    pick,
    remove,
    reorder: setPaths,
    processAnother,
    setUnsaved,
    leaving,
    stay: () => setLeaving(null),
    leave: () => {
      setLeaving(null);
      leaving?.action();
    },
    replacing,
    keepFile: () => setReplacing(null),
    replaceFile: () => {
      setReplacing(null);
      job.reset();
      if (replacing) setPaths([replacing]);
    },
  };
}

export type ToolSession = ReturnType<typeof useToolSession>;
