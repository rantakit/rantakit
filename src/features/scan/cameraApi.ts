import { invoke } from "@tauri-apps/api/core";
import { nameFile, pickImages } from "@/features/compress/imageApi";
import { currentTranslator } from "@/i18n";

/**
 * Scan Dokumen's camera (D3), on Android; see `src-tauri/src/camera.rs`.
 * "ask" until Android stops asking, then "refused": only Settings can allow
 * it from there.
 */
export type CameraPermission = "granted" | "ask" | "refused";

export const cameraPermission = () => invoke<CameraPermission>("camera_permission").catch(() => "refused" as const);

/** Android's prompt, after Rantakit has said why; whether the camera may be used. */
export const askCameraPermission = () => invoke<boolean>("ask_camera_permission").catch(() => false);

/** Rantakit's page in the phone's Settings. */
export const openAppSettings = () => invoke("open_app_settings").catch(() => {});

/**
 * Opens the camera; the photos taken, in order. When "Dari Galeri" closed
 * it, the photos picked from Gallery follow them.
 */
/** Photos taken since Rantakit opened, for their names. */
let shots = 0;

export async function takePhotos(): Promise<string[]> {
  const { t } = currentTranslator();
  // CameraActivity.kt fills in "{count}" itself.
  const count = "{count}";
  const words = {
    preview: t("camera.preview"),
    noCamera: t("camera.noCamera"),
    taken: t("camera.taken", { count }),
    failed: t("camera.failed"),
    flashOn: t("camera.flashOn"),
    flashOff: t("camera.flashOff"),
    pages: t("camera.pages", { count }),
    close: t("camera.close"),
    hint: t("camera.hint"),
    gallery: t("camera.gallery"),
    shutter: t("camera.shutter"),
    done: t("camera.done"),
  };
  const taken = await invoke<{ paths: string[]; gallery: boolean }>("take_photos", { words }).catch(() => ({
    paths: [],
    gallery: false,
  }));
  // "Jepretan 3" rather than the file's name, which is only a time.
  for (const path of taken.paths) nameFile(path, t("camera.name", { count: ++shots }));
  return taken.gallery ? [...taken.paths, ...(await pickImages())] : taken.paths;
}

/** Removes photos the camera took, once they are no longer in a Task; others are left alone. */
export const discardPhotos = (paths: string[]) => {
  if (paths.length > 0) invoke("discard_photos", { paths }).catch(() => {});
};
