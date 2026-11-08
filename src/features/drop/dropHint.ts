import type { ShippedTool } from "@/features/tool/tools";
import { toolDef } from "@/features/tool/tools";
import type { Translator } from "@/i18n";
import { filesNoun, type DropMatch } from "./dropMatch";

interface DropState {
  /** A Task is running, so drops are ignored. */
  busy: boolean;
  /** The Tool page on screen, which takes dropped files directly; null on Home and History. */
  openTool: ShippedTool | null;
  /** Null while the dragged files are being checked. */
  match: DropMatch | null;
  count: number;
  /** Chosen with "Buka" or shared rather than dragged, so a hint says to choose. */
  chosen?: boolean;
}

/** What the drop overlay says while files are dragged over the window. */
export function dropHint({ t, count }: Translator, { busy, openTool, match, count: n, chosen }: DropState) {
  if (busy) return { canDrop: false, title: t("drop.busy") };
  if (!match) return { canDrop: true, title: t("drop.here") };

  const files = count(`files.${filesNoun(match)}`, n);
  if (openTool) {
    if (match.status === "fits" && match.tools.includes(openTool)) {
      return { canDrop: true, title: t("drop.addTo", { tool: t(`tool.${openTool}`) }), detail: files };
    }
    if (match.status === "unsupported" || match.status === "fits") {
      return {
        canDrop: false,
        title: t("error.unsupported-format"),
        detail: t("error.accepted", { formats: t(`formats.${toolDef(openTool).noun}`) }),
      };
    }
  }

  switch (match.status) {
    case "fits":
      return { canDrop: true, title: t("drop.here"), detail: files };
    case "mixed":
      return { canDrop: false, title: t("drop.mixed"), detail: t(chosen ? "drop.mixed.choose" : "drop.mixed.hint") };
    case "unsupported":
      return { canDrop: false, title: t("drop.unsupported"), detail: t("drop.accepted") };
    case "oldOffice":
      return { canDrop: false, title: t("drop.oldOffice"), detail: t("drop.oldOffice.detail") };
  }
}
