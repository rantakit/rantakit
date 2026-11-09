import { ShieldCheck } from "@keyline-icons/react";
import { pickImages } from "@/features/compress/imageApi";
import type { TaskResult, ToolOptions } from "@/features/tool/taskApi";
import type { FileInfo } from "@/features/tool/fileInfo";
import { defineTool } from "@/features/tool/toolDef";
import { useT, type Translator } from "@/i18n";

/** Hapus Info Lokasi has no options. */
type CleanInfoSettings = Record<string, never>;

const NO_SETTINGS: CleanInfoSettings = {};
const useCleanInfoSettings = () => [NO_SETTINGS, () => {}] as const;

/** What goes and what stays, said once above the button. */
function Main() {
  const { t } = useT();
  return (
    <div className="flex items-start gap-3 rounded-2xl border border-border bg-surface p-4">
      <span className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-brand-soft text-brand-strong">
        <ShieldCheck aria-hidden className="size-5" />
      </span>
      <p className="text-small text-text-muted">{t("cleanInfo.what")}</p>
    </div>
  );
}

/** A photo with nothing but its orientation keeps no Result: "Sudah bersih, tidak diubah". */
function leavesUnchanged(options: ToolOptions, file: FileInfo) {
  return options.tool === "hapus-info-lokasi" && file.found !== null && !hasInfo(file.found);
}

function hasInfo(found: NonNullable<FileInfo["found"]>) {
  return found.location || found.camera || found.other;
}

/** "Ada lokasi · Ada info kamera", or "Ada info foto lain" when it holds neither. */
function fileNote(_options: ToolOptions, file: FileInfo, { t }: Translator) {
  const found = file.found;
  if (!found || !hasInfo(found)) return null;
  const notes = [found.location && t("cleanInfo.location"), found.camera && t("cleanInfo.camera")].filter(Boolean);
  return notes.length > 0 ? notes.join(" · ") : t("cleanInfo.other");
}

/** "Info dihapus dari 3 foto." */
function summary(results: TaskResult[], { t, count }: Translator) {
  return t("cleanInfo.result", { photos: count("files.photo", results.length) });
}

export const hapusInfoLokasi = defineTool({
  noun: "photo",
  presets: false,
  pick: pickImages,
  useSettings: useCleanInfoSettings,
  Main,
  toRequest: () => ({ tool: "hapus-info-lokasi" }),
  leavesUnchanged,
  fileNote,
  summary,
});
