import { useState } from "react";
import type { ToolId } from "@/features/home/tools";
import { isShipped, type ShippedTool } from "@/features/tool/tools";
import { useT } from "@/i18n";
import { useFileDrop } from "@/lib/fileDrop";
import { dropHint } from "./dropHint";
import { matchFiles, useDropMatch, type DropMatch } from "./dropMatch";
import { pickAnyFiles } from "./pickFiles";
import { useSharedFiles } from "./sharedFiles";

/** Why files were not taken, in the drop overlay's words. */
export interface Refusal {
  title: string;
  detail?: string;
}

/** Files waiting for the user to choose a Tool. */
interface Dropped {
  paths: string[];
  match: Extract<DropMatch, { status: "fits" }>;
}

interface IncomingOptions {
  /** A Task is running, so new files are not taken. */
  busy: boolean;
  /** The Tool page on screen, which takes files directly; null on Home and History. */
  openTool: ShippedTool | null;
  /** Adds files to the open Tool page; says why when they can't go on it. */
  addToTool: (paths: string[]) => Refusal | null;
  /** Opens a Tool's page with these files, once the user chose it. */
  openWith: (tool: ShippedTool, paths: string[]) => void;
  /** Android share: runs `then` after leaving the page, asking first if files would be lost. */
  leaveFor: (then: () => void) => void;
}

/**
 * Files that come from outside a Tool page: dropped on the window, chosen with
 * "Buka", or shared from another app on Android. On Home they ask for a Tool;
 * on a Tool page they are added to it. Render `IncomingFilesDialogs` with the
 * result.
 */
export function useIncomingFiles({ busy, openTool, addToTool, openWith, leaveFor }: IncomingOptions) {
  const tr = useT();
  const [dropped, setDropped] = useState<Dropped | null>(null);
  /** Why the files were not taken. */
  const [refused, setRefused] = useState<Refusal | null>(null);
  /** A share that came while a Task was running, and was not taken. */
  const [shareBusy, setShareBusy] = useState(false);

  /** Adds files to the open Tool page, or says why they can't go on it. */
  const add = (paths: string[]) => {
    const refusal = addToTool(paths);
    if (refusal) setRefused(refusal);
  };

  /**
   * Ask for a Tool, or add the files to the open one. Files that don't fit
   * come back with the reason, in the drop overlay's words.
   */
  const receive = async (received: string[]): Promise<Refusal | null> => {
    if (busy || received.length === 0) return null;
    const match = await matchFiles(received);
    // Only "Buka" shows the hint; a drop was explained while dragging.
    const hint = dropHint(tr, { busy: false, openTool, match, count: received.length, chosen: true });
    if (!hint.canDrop || match.status !== "fits") return hint;
    if (!openTool) setDropped({ paths: received, match });
    else add(received);
    return null;
  };

  /** A drop that doesn't fit was already explained by the overlay while dragging. */
  const dragging = useFileDrop((paths) => void receive(paths));
  const draggedMatch = useDropMatch(dragging);

  const openFiles = async () => {
    const hint = await receive(await pickAnyFiles({ documents: true }));
    if (hint) setRefused(hint);
  };

  /** Shared from another app: the Tools that fit, as for "Buka" on Home, after leaving the page. */
  const share = (shared: string[]) => {
    if (busy) return setShareBusy(true);
    leaveFor(async () => {
      const match = await matchFiles(shared);
      const hint = dropHint(tr, { busy: false, openTool: null, match, count: shared.length, chosen: true });
      if (!hint.canDrop || match.status !== "fits") setRefused(hint);
      else setDropped({ paths: shared, match });
    });
  };
  useSharedFiles(share);

  const chooseTool = (id: ToolId) => {
    if (isShipped(id) && dropped) openWith(id, dropped.paths);
    setDropped(null);
  };

  return {
    /** What the drop overlay says while files are dragged over the window; null when none are. */
    overlay: dragging
      ? dropHint(tr, { busy, openTool, match: draggedMatch, count: dragging.length })
      : null,
    dropped,
    refused,
    shareBusy,
    add,
    openFiles,
    chooseTool,
    closeDropped: () => setDropped(null),
    closeRefused: () => setRefused(null),
    closeShareBusy: () => setShareBusy(false),
  };
}

export type IncomingFiles = ReturnType<typeof useIncomingFiles>;
