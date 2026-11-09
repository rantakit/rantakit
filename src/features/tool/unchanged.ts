import type { ToolOptions } from "./taskApi";
import type { Translator } from "@/i18n";

/** Why a file was left as it is: "Sudah JPG, tidak diubah". Switches on the request, which keeps the engine's names. */
export function unchangedNote(t: Translator["t"], options: ToolOptions): string {
  switch (options.tool) {
    case "ubah-format-foto":
      return t("unchanged.format", { format: options.to === "jpeg" ? "JPG" : "PNG" });
    case "ubah-ukuran-foto":
      return t("unchanged.size");
    case "hapus-info-lokasi":
      return t("unchanged.clean");
    case "kompres-dokumen":
      return t("unchanged.size");
    default:
      return t("error.unchanged");
  }
}
