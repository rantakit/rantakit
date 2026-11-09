import { fileName } from "@/features/compress/imageApi";
import { useT } from "@/i18n";
import { pageNoun } from "./fileInfo";
import { LeaveDialog } from "./LeaveDialog";
import { toolDef, type ShippedTool } from "./tools";
import type { ToolSession } from "./useToolSession";

interface LeaveToolDialogProps {
  /** Whether leaving stops a running Task, rather than losing the files chosen. */
  working: boolean;
  tool: ShippedTool | null;
  paths: string[];
  onLeave: () => void;
  onStay: () => void;
}

/**
 * Asked before leaving a Tool page: a running Task would stop, or the files
 * on it, a work area's changes, or Scan Dokumen's photos would be lost.
 */
export function LeaveToolDialog({ working, tool, paths, onLeave, onStay }: LeaveToolDialogProps) {
  const { t } = useT();
  const noun = tool ? pageNoun(toolDef(tool).noun, paths) : "photo";
  const words = working
    ? {
        title: t("leave.working.title"),
        body: t(`leave.working.body.${noun}`),
        leaveLabel: t("leave.working.stop"),
        stayLabel: t("leave.working.keep"),
      }
    : {
        title: t("leave.title"),
        body:
          tool && toolDef(tool).Workspace && paths[0]
            ? t("arrange.replace.body", { name: fileName(paths[0]) })
            : tool && toolDef(tool).Source
              ? t("leave.body.scan")
              : t(`leave.body.${noun}`),
        leaveLabel: t("leave.leave"),
        stayLabel: t("leave.stay"),
      };
  return <LeaveDialog {...words} onStay={onStay} onLeave={onLeave} />;
}

/** The questions of `useToolSession`: leave the page, or replace the file in a work area. */
export function ToolSessionDialogs({ session }: { session: ToolSession }) {
  const { tool, paths, leaving, replacing } = session;
  return (
    <>
      {leaving && (
        <LeaveToolDialog
          working={leaving.working}
          tool={tool}
          paths={paths}
          onStay={session.stay}
          onLeave={session.leave}
        />
      )}
      {replacing && paths[0] && (
        <ReplaceFileDialog current={paths[0]} onKeep={session.keepFile} onReplace={session.replaceFile} />
      )}
    </>
  );
}

interface ReplaceFileDialogProps {
  /** The file in the work area now, whose changes would be lost. */
  current: string;
  onReplace: () => void;
  onKeep: () => void;
}

/** "Ganti PDF?": another file chosen for a Tool with its own work area that has changes. */
export function ReplaceFileDialog({ current, onReplace, onKeep }: ReplaceFileDialogProps) {
  const { t } = useT();
  return (
    <LeaveDialog
      title={t("arrange.replace.title")}
      body={t("arrange.replace.body", { name: fileName(current) })}
      leaveLabel={t("arrange.replace")}
      stayLabel={t("arrange.replace.cancel")}
      onStay={onKeep}
      onLeave={onReplace}
    />
  );
}
