import { AlertDialog } from "@/components/ui/AlertDialog";
import { Modal } from "@/components/ui/Modal";
import { useT } from "@/i18n";
import type { PermissionGate } from "./usePermissionGate";

/** The dialogs of `usePermissionGate`: why a permission is asked for, or that it was refused. */
export function PermissionDialogs({ gate }: { gate: PermissionGate }) {
  const { t } = useT();
  const { saving, notifying } = gate;
  return (
    <>
      {saving && (
        <Modal isOpen isDismissable onOpenChange={(open) => !open && gate.closeSaving()}>
          <AlertDialog
            title={t(`saving.${saving.status}.title`)}
            actionLabel={t(saving.status === "ask" ? "saving.ask.continue" : "saving.refused.retry")}
            cancelLabel={t(saving.status === "ask" ? "common.cancel" : "common.close")}
            onAction={() => gate.askToSave(saving)}
          >
            {t(`saving.${saving.status}.body`)}
          </AlertDialog>
        </Modal>
      )}
      {notifying && (
        <Modal isOpen isDismissable onOpenChange={(open) => !open && gate.closeNotifying()}>
          <AlertDialog
            title={t("notifying.title")}
            actionLabel={t("notifying.continue")}
            cancelLabel={t("common.cancel")}
            onAction={() => gate.askToNotify(notifying)}
          >
            {t("notifying.body")}
          </AlertDialog>
        </Modal>
      )}
    </>
  );
}
