import { AlertDialog } from "@/components/ui/AlertDialog";
import { Modal } from "@/components/ui/Modal";
import { useT } from "@/i18n";
import { DropDialog } from "./DropDialog";
import { DropOverlay } from "./DropOverlay";
import type { IncomingFiles } from "./useIncomingFiles";

/** The overlay and dialogs of `useIncomingFiles`. */
export function IncomingFilesDialogs({ incoming }: { incoming: IncomingFiles }) {
  const { t } = useT();
  const { overlay, dropped, refused, shareBusy } = incoming;
  return (
    <>
      {overlay && <DropOverlay {...overlay} />}
      {dropped && (
        <DropDialog
          paths={dropped.paths}
          match={dropped.match}
          onChoose={incoming.chooseTool}
          onClose={incoming.closeDropped}
        />
      )}
      {refused && (
        <Modal isOpen isDismissable onOpenChange={(open) => !open && incoming.closeRefused()}>
          <AlertDialog
            title={refused.title}
            actionLabel={t("drop.pickAgain")}
            cancelLabel={t("common.close")}
            onAction={incoming.openFiles}
          >
            {refused.detail}
          </AlertDialog>
        </Modal>
      )}
      {shareBusy && (
        <Modal isOpen isDismissable onOpenChange={(open) => !open && incoming.closeShareBusy()}>
          <AlertDialog title={t("share.busy.title")} cancelLabel={t("common.close")}>
            {t("share.busy.body")}
          </AlertDialog>
        </Modal>
      )}
    </>
  );
}
