import { useEffect, useRef, useState } from "react";
import { Camera, CameraOff, Image, Settings } from "@keyline-icons/react";
import { RadioCards } from "@/components/ui/RadioCards";
import { Button } from "@/components/ui/Button";
import { pickImages } from "@/features/compress/imageApi";
import type { TaskResult } from "@/features/tool/taskApi";
import { OptionField } from "@/features/tool/OptionField";
import { defineTool, type OptionsProps, type SourceProps } from "@/features/tool/toolDef";
import {
  CropHost,
  NoMarginOption,
  PAPERS,
  PaperField,
  PdfNote,
  usePhotoRow,
  type Paper,
} from "@/features/photo-pdf/PhotoPdfOptions";
import { photoEditsFor } from "@/features/photo-pdf/photoEdits";
import { useT, type Translator } from "@/i18n";
import { today } from "@/lib/format";
import { useStoredState } from "@/lib/storedState";
import {
  askCameraPermission,
  cameraPermission,
  discardPhotos,
  openAppSettings,
  takePhotos,
  type CameraPermission,
} from "./cameraApi";

const LOOKS = ["clean", "original"] as const;
type Look = (typeof LOOKS)[number];

export interface ScanSettings {
  /** "Tampilan": "Bersihkan" or "Warna asli". */
  look: Look;
  /** "Ukuran kertas". */
  paper: Paper;
  /** "Tanpa margin", in Advanced options; A4 and F4 only. */
  noMargin: boolean;
}

function parse(stored: unknown): ScanSettings {
  const s = (stored ?? {}) as Partial<Record<keyof ScanSettings, unknown>>;
  return {
    look: LOOKS.find((look) => look === s.look) ?? "clean",
    paper: PAPERS.find((paper) => paper === s.paper) ?? "a4",
    noMargin: s.noMargin === true,
  };
}

function useScanSettings() {
  return useStoredState("rantakit.scanDokumen", parse);
}

function Main({ settings, onChange, paths }: OptionsProps<ScanSettings>) {
  const { t, count } = useT();
  return (
    <>
      <OptionField label={t("scan.look")}>
        <RadioCards
          label={t("scan.look")}
          value={settings.look}
          options={LOOKS.map((look) => ({
            id: look,
            label: t(`scan.look.${look}`),
            description: t(`scan.look.${look}.description`),
          }))}
          onChange={(look) => onChange({ look })}
        />
      </OptionField>
      <PaperField paper={settings.paper} onChange={(paper) => onChange({ paper })} />
      {paths.length > 0 && <PdfNote files={count("pages", paths.length)} paper={settings.paper} />}
      <CropHost paths={paths} />
    </>
  );
}

/** "3 halaman → 1 PDF." */
function summary(results: TaskResult[], { t, count }: Translator) {
  return t("scan.result", { pages: count("pages", results[0]?.pages ?? results.length) });
}

/**
 * Where Scan Dokumen's pages come from (D3, Q10). Opening the Tool goes
 * straight to the camera, after saying why it is needed the first time.
 * With pages already taken, it is "Scan halaman lagi" and "Dari Galeri".
 */
function ScanSource({ empty, onAdd }: SourceProps) {
  const { t } = useT();
  const [permission, setPermission] = useState<CameraPermission | null>(null);
  const opened = useRef(false);

  const scan = async () => onAdd(await takePhotos());
  const fromGallery = async () => onAdd(await pickImages());

  useEffect(() => {
    let current = true;
    const check = () =>
      cameraPermission().then((state) => {
        if (!current) return;
        setPermission(state);
        if (state === "granted" && empty && !opened.current) {
          opened.current = true;
          scan();
        }
      });
    check();
    // Back from the phone's Settings, where the camera may have been allowed.
    const onVisible = () => document.visibilityState === "visible" && check();
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      current = false;
      document.removeEventListener("visibilitychange", onVisible);
    };
    // Checked once per visit; `empty` only decides whether the camera opens by itself.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const allow = async () => {
    if (await askCameraPermission()) {
      setPermission("granted");
      opened.current = true;
      scan();
    } else {
      setPermission(await cameraPermission());
    }
  };

  const galleryButton = (variant: "primary" | "secondary", label: "scan.gallery" | "scan.permission.gallery") => (
    <Button variant={variant} size={empty ? undefined : "row"} onPress={fromGallery}>
      <Image aria-hidden className="size-5" /> {t(label)}
    </Button>
  );

  if (!empty) {
    return (
      <>
        {permission !== "refused" && (
          <Button variant="secondary" size="row" isDisabled={permission === null} onPress={permission === "ask" ? allow : scan}>
            <Camera aria-hidden className="size-5" /> {t("scan.more")}
          </Button>
        )}
        {galleryButton("secondary", "scan.gallery")}
      </>
    );
  }

  const screen =
    permission === null
      ? null
      : permission === "granted"
        ? {
            icon: Camera,
            title: t("scan.start.title"),
            body: t("scan.start.body"),
            action: (
              <Button onPress={scan}>
                <Camera aria-hidden className="size-5" /> {t("scan.start.camera")}
              </Button>
            ),
            gallery: galleryButton("secondary", "scan.gallery"),
          }
        : permission === "ask"
          ? {
              icon: Camera,
              title: t("scan.permission.title"),
              body: t("scan.permission.body"),
              action: <Button onPress={allow}>{t("scan.permission.allow")}</Button>,
              gallery: galleryButton("secondary", "scan.permission.gallery"),
            }
          : {
              icon: CameraOff,
              title: t("scan.refused.title"),
              body: t("scan.refused.body"),
              action: (
                <Button onPress={openAppSettings}>
                  <Settings aria-hidden className="size-5" /> {t("scan.refused.settings")}
                </Button>
              ),
              gallery: galleryButton("secondary", "scan.permission.gallery"),
            };

  return (
    <div className="mx-5 mb-5 flex min-h-80 flex-1 flex-col items-center justify-center gap-5 rounded-[20px] border border-border bg-surface p-8 text-center md:min-h-0">
      {screen ? (
        <>
          <span className="flex size-18 items-center justify-center rounded-[20px] bg-brand-soft text-brand-strong">
            <screen.icon aria-hidden className="size-9" />
          </span>
          <div className="flex max-w-80 flex-col gap-1.5">
            <h2 className="font-display text-section">{screen.title}</h2>
            <p className="text-small text-text-muted">{screen.body}</p>
          </div>
          <div className="flex w-full max-w-72 flex-col gap-2.5">
            {screen.action}
            {screen.gallery}
          </div>
        </>
      ) : (
        <p role="status" className="text-small text-text-muted">
          {t("scan.checking")}
        </p>
      )}
    </div>
  );
}

export const scanDokumen = defineTool({
  noun: "photo",
  counts: "pages",
  ordered: true,
  presets: false,
  pick: pickImages,
  Source: ScanSource,
  discard: discardPhotos,
  useSettings: useScanSettings,
  Main,
  Advanced: NoMarginOption,
  showAdvanced: (settings) => settings.paper !== "photo",
  toRequest: (settings, _files, paths) => ({
    tool: "scan-dokumen",
    date: today(),
    paper: settings.paper,
    margin: settings.paper === "photo" || !settings.noMargin,
    edits: photoEditsFor(paths),
    look: settings.look,
  }),
  summary,
  useRow: (path, _paths, settings) => usePhotoRow(path, settings.look === "clean"),
});
