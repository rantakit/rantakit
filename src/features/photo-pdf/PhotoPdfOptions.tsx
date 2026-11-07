import { CircleAlert, CircleCheck, Scan } from "@keyline-icons/react";
import { RadioCards } from "@/components/ui/RadioCards";
import { Button } from "@/components/ui/Button";
import { Switch } from "@/components/ui/Switch";
import { fileName, pickImages } from "@/features/compress/imageApi";
import type { TaskResult } from "@/features/compress/taskApi";
import { OptionField } from "@/features/tool/OptionField";
import { defineTool, type OptionsProps, type RowExtras } from "@/features/tool/toolDef";
import { useThumbnail } from "@/features/tool/fileInfo";
import { useT, type Translator } from "@/i18n";
import { today } from "@/lib/format";
import { useStoredState } from "@/lib/storedState";
import { CropEditor } from "./CropEditor";
import {
  isUnedited,
  openCropEditor,
  photoEditsFor,
  setPhotoEdit,
  useCropEditing,
  useEditedPreview,
  usePhotoEdits,
} from "./photoEdits";

const PAPERS = ["a4", "f4", "photo"] as const;
type Paper = (typeof PAPERS)[number];

export interface PhotoPdfSettings {
  /** "Ukuran kertas". */
  paper: Paper;
  /** "Tanpa margin", in Advanced options; A4 and F4 only. */
  noMargin: boolean;
}

function parse(stored: unknown): PhotoPdfSettings {
  const s = (stored ?? {}) as Partial<Record<keyof PhotoPdfSettings, unknown>>;
  return {
    paper: PAPERS.find((paper) => paper === s.paper) ?? "a4",
    noMargin: s.noMargin === true,
  };
}

function usePhotoPdfSettings() {
  return useStoredState("rantakit.jpgKePdf", parse);
}

function Main({ settings, onChange, paths }: OptionsProps<PhotoPdfSettings>) {
  const tr = useT();
  const { t, count } = tr;
  return (
    <>
      <OptionField label={t("photoPdf.paper")}>
        <RadioCards
          label={t("photoPdf.paper")}
          value={settings.paper}
          options={PAPERS.map((paper) => ({
            id: paper,
            label: t(`photoPdf.paper.${paper}`),
            description: t(`photoPdf.paper.${paper}.description`),
          }))}
          onChange={(paper) => onChange({ paper })}
        />
      </OptionField>
      {paths.length > 0 && (
        <p className="flex items-start gap-2.5 rounded-xl bg-badge-bg px-3.5 py-3 text-small text-text-muted">
          <CircleCheck aria-hidden className="mt-px size-[18px] shrink-0 text-brand-strong" />
          <span>
            {t("photoPdf.note", { files: count("files.photo", paths.length), name: `scan ${today()}.pdf` })}{" "}
            {t(settings.paper === "photo" ? "photoPdf.note.photo" : "photoPdf.note.paper")}
          </span>
        </p>
      )}
      <CropHost paths={paths} />
    </>
  );
}

function Advanced({ settings, onChange }: OptionsProps<PhotoPdfSettings>) {
  const { t } = useT();
  return (
    <div className="flex items-start justify-between gap-6">
      <div className="flex flex-col gap-0.5">
        <span className="text-label">{t("photoPdf.noMargin")}</span>
        <span className="text-small text-text-muted">{t("photoPdf.noMargin.description")}</span>
      </div>
      <Switch
        aria-label={t("photoPdf.noMargin")}
        isSelected={settings.noMargin}
        onChange={(noMargin) => onChange({ noMargin })}
      />
    </div>
  );
}

/** "3 foto → 1 PDF, 3 halaman." Every photo's outcome points at the same Result. */
function summary(results: TaskResult[], { t, count }: Translator) {
  const pages = results[0]?.pages ?? results.length;
  return t("photoPdf.result", { files: count("files.photo", results.length), pages: count("pages", pages) });
}

/** Each row: the photo as it will go on its page, how it was cropped, and "Atur potongan" (D1). */
function usePhotoRow(path: string): RowExtras {
  const { t } = useT();
  const { edit, source } = usePhotoEdits([path])[path];
  const plain = useThumbnail(path, 96);
  const cropped = useEditedPreview(path, source === "finding" || isUnedited(edit) ? null : edit, 96);
  const name = fileName(path);

  const note =
    source === "finding"
      ? t("crop.finding")
      : source === "notFound"
        ? t("crop.notFound")
        : source === "found"
          ? t("crop.auto")
          : edit.corners
            ? t("crop.set")
            : t("crop.whole.used");
  const Icon = source === "notFound" ? CircleAlert : Scan;

  return {
    thumbnail: isUnedited(edit) ? plain : cropped,
    warn: source === "notFound",
    note: (
      <>
        {source !== "finding" && <Icon aria-hidden className="mt-0.5 size-4 shrink-0 self-start" />}
        {/* Wraps rather than cut off, so it is read whole. */}
        <span className="min-w-0">{note}</span>
      </>
    ),
    action: (
      <Button
        variant="secondary"
        size="row"
        aria-label={t("crop.open.label", { name })}
        className="shrink-0 @max-lg:size-11 @max-lg:min-h-0 @max-lg:border-0 @max-lg:bg-transparent @max-lg:px-0 @max-lg:text-text-muted"
        isDisabled={source === "finding"}
        onPress={() => openCropEditor(path)}
      >
        <Scan aria-hidden className="size-5" />
        <span className="@max-lg:hidden">{t("crop.open")}</span>
      </Button>
    ),
  };
}

/**
 * The crop editor for the photo whose "Atur potongan" was pressed. It lives
 * outside the file list, so its keys and presses stay its own.
 */
function CropHost({ paths }: { paths: string[] }) {
  const path = useCropEditing();
  const edits = usePhotoEdits(path ? [path] : []);
  if (!path || !paths.includes(path)) return null;
  return (
    <CropEditor
      key={path}
      path={path}
      name={fileName(path)}
      position={paths.indexOf(path) + 1}
      count={paths.length}
      edit={edits[path].edit}
      onCancel={() => openCropEditor(null)}
      onDone={(next) => {
        setPhotoEdit(path, next);
        openCropEditor(null);
      }}
    />
  );
}

export const jpgKePdf = defineTool({
  noun: "photo",
  ordered: true,
  presets: false,
  pick: pickImages,
  useSettings: usePhotoPdfSettings,
  Main,
  Advanced,
  showAdvanced: (settings) => settings.paper !== "photo",
  toRequest: (settings, _files, paths) => ({
    tool: "jpg-ke-pdf",
    date: today(),
    paper: settings.paper,
    margin: settings.paper === "photo" || !settings.noMargin,
    edits: photoEditsFor(paths),
  }),
  summary,
  useRow: usePhotoRow,
});
