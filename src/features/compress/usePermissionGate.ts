import { useState } from "react";
import {
  askNotificationPermission,
  askToSaveResults,
  canSaveResults,
  notificationPermission,
  type ToolOptions,
} from "@/features/tool/taskApi";

/** A Task waiting for the user to answer a permission dialog. */
interface Waiting {
  paths: string[];
  options: ToolOptions;
}

/**
 * Asks for what Android needs before a Task: the storage permission on
 * Android 8 and 9, and notifications once on Android 13 and later, each after
 * saying why. Elsewhere `request` starts the Task at once. Render
 * `PermissionDialogs` with the result.
 */
export function usePermissionGate(start: (paths: string[], options: ToolOptions) => void) {
  /** Why the storage permission is asked for, or that it was refused. */
  const [saving, setSaving] = useState<(Waiting & { status: "ask" | "refused" }) | null>(null);
  /** Why notifications are asked for, once, before the first Task. */
  const [notifying, setNotifying] = useState<Waiting | null>(null);

  const request = async (paths: string[], options: ToolOptions) => {
    if (!(await canSaveResults())) setSaving({ status: "ask", paths, options });
    else if ((await notificationPermission()) === "ask") setNotifying({ paths, options });
    else start(paths, options);
  };

  const askToSave = async ({ paths, options }: Waiting) => {
    if (await askToSaveResults()) start(paths, options);
    else setSaving({ status: "refused", paths, options });
  };

  /** The Task starts whatever the answer; without the permission only its notification is hidden. */
  const askToNotify = async ({ paths, options }: Waiting) => {
    await askNotificationPermission();
    start(paths, options);
  };

  return {
    request,
    saving,
    notifying,
    askToSave,
    askToNotify,
    closeSaving: () => setSaving(null),
    closeNotifying: () => setNotifying(null),
  };
}

export type PermissionGate = ReturnType<typeof usePermissionGate>;
