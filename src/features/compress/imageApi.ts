import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { currentTranslator } from "@/i18n";
import { isAndroidApp } from "@/lib/platform";

export const IMAGE_EXTENSIONS = ["jpg", "jpeg", "png"];
export const IMAGE_MIME_TYPES = ["image/jpeg", "image/png"];

/** Names of files chosen or shared on Android, by their `content://` address. */
const chosenNames = new Map<string, string>();

/** A file's name: the last segment of a Windows or POSIX path, or the name Android gave a chosen file. */
export const fileName = (path: string) => chosenNames.get(path) ?? path.split(/[\\/]/).pop() ?? path;

/** Names a file Rantakit made itself, such as a photo the camera took: "Jepretan 1". */
export const nameFile = (path: string, name: string) => void chosenNames.set(path, name);

/**
 * Looks up the names of files chosen or shared on Android, whose `content://`
 * addresses do not contain them, so that `fileName` can show them.
 */
export async function withNames(paths: string[]): Promise<string[]> {
  if (paths.length === 0) return paths;
  const names = await invoke<(string | null)[]>("file_names", { paths }).catch(() => []);
  paths.forEach((path, i) => names[i] && chosenNames.set(path, names[i]));
  return paths;
}

/**
 * Android's system file picker, which keeps each file's real name. The
 * dialog plugin would open the Photo Picker for photos, which hides them.
 */
export async function pickOnAndroid(mimeTypes: string[]): Promise<string[]> {
  return withNames(await invoke<string[]>("pick_files", { mimeTypes }).catch(() => []));
}

/** Ask the user for photos. Returns an empty list when cancelled or outside Tauri. */
export async function pickImages(): Promise<string[]> {
  if (!isTauri()) return import.meta.env.DEV ? (await import("@/lib/devMocks")).devPick("photo") : [];
  if (isAndroidApp) return pickOnAndroid(IMAGE_MIME_TYPES);

  const { t } = currentTranslator();
  const selected = await open({
    title: t("picker.photos"),
    multiple: true,
    directory: false,
    filters: [{ name: t("picker.photos.filter"), extensions: IMAGE_EXTENSIONS }],
  });
  return selected ?? [];
}
