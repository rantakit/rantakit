import { isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { IMAGE_EXTENSIONS, IMAGE_MIME_TYPES, pickOnAndroid } from "@/features/compress/imageApi";
import { DOCUMENT_EXTENSIONS, DOCUMENT_MIME_TYPES } from "@/features/compress-documents/documentApi";
import { currentTranslator } from "@/i18n";
import { isAndroidApp } from "@/lib/platform";

/**
 * Ask the user for photos or PDFs, for a Tool page that takes either. With
 * `documents`, Word, Excel, and PowerPoint files too: "Buka" and Home's
 * "Pilih file" take any file a Tool opens, then go the same way as a drop.
 * Returns an empty list when cancelled or outside Tauri.
 */
export async function pickAnyFiles({ documents = false } = {}): Promise<string[]> {
  if (!isTauri()) return import.meta.env.DEV ? (await import("@/lib/devMocks")).devPick("photo") : [];
  if (isAndroidApp) {
    return pickOnAndroid([...IMAGE_MIME_TYPES, "application/pdf", ...(documents ? DOCUMENT_MIME_TYPES : [])]);
  }

  const { t } = currentTranslator();
  const selected = await open({
    title: t("picker.files"),
    multiple: true,
    directory: false,
    filters: [
      {
        name: t(documents ? "picker.files.filter.documents" : "picker.files.filter"),
        extensions: [...IMAGE_EXTENSIONS, "pdf", ...(documents ? DOCUMENT_EXTENSIONS : [])],
      },
    ],
  });
  return selected ?? [];
}
