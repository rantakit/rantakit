import { isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { pickOnAndroid } from "@/features/compress/imageApi";
import { currentTranslator } from "@/i18n";
import { isAndroidApp } from "@/lib/platform";

/** Word, Excel, and PowerPoint, with their macro-enabled variants. */
export const DOCUMENT_EXTENSIONS = ["docx", "xlsx", "pptx", "docm", "xlsm", "pptm"];

export const DOCUMENT_MIME_TYPES = [
  "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
  "application/vnd.openxmlformats-officedocument.presentationml.presentation",
  "application/vnd.ms-word.document.macroEnabled.12",
  "application/vnd.ms-excel.sheet.macroEnabled.12",
  "application/vnd.ms-powerpoint.presentation.macroEnabled.12",
];

/** Ask the user for Word, Excel, or PowerPoint files. Empty when cancelled or outside Tauri. */
export async function pickDocuments(): Promise<string[]> {
  if (!isTauri()) return import.meta.env.DEV ? (await import("@/lib/devMocks")).devPick("document") : [];
  if (isAndroidApp) return pickOnAndroid(DOCUMENT_MIME_TYPES);

  const selected = await open({
    title: currentTranslator().t("files.pick.document"),
    multiple: true,
    directory: false,
    filters: [{ name: "Word, Excel, PowerPoint", extensions: DOCUMENT_EXTENSIONS }],
  });
  return selected ?? [];
}
