import { useCallback, useMemo } from "react";
import { pickAnyFiles } from "@/features/drop/pickFiles";
import { pdfKeJpg } from "@/features/pdf-to-jpg/PdfToJpgOptions";
import { nounOf, pageNoun, type FileInfo, type Noun } from "@/features/tool/fileInfo";
import { defineTool, type OptionsProps, type ToolDef } from "@/features/tool/toolDef";
import { ConvertNote, photosToJpg } from "./ConvertOptions";

/** Both halves' settings, each kept where its own page kept it before 0.4.1. */
interface Halves<P, D> {
  photo: P;
  pdf: D;
}

type HalvesPatch<P, D> = { photo?: Partial<P>; pdf?: Partial<D> };

/** The noun of chosen files that have been read; null while none are. */
function readNoun(files: FileInfo[]): "photo" | "pdf" | null {
  if (files.length === 0) return null;
  return files[0].kind === "pdf" ? "pdf" : "photo";
}

/**
 * One Tool page for two kinds of file, one kind per Task (X5): the options,
 * the Task, and its summary follow the files on the page, from `photo` for
 * photos and `pdf` for PDFs. `isPdf` tells a Preset saved before the halves
 * were joined, kept flat, that belongs to `pdf`.
 */
function byNoun<P extends object, D extends object>(
  photo: ToolDef<P>,
  pdf: ToolDef<D>,
  isPdf: (flat: object) => boolean,
): ToolDef<Halves<P, D>> {
  type S = Halves<P, D>;

  function useSettings() {
    const [photoSettings, updatePhoto] = photo.useSettings();
    const [pdfSettings, updatePdf] = pdf.useSettings();
    const settings = useMemo(() => ({ photo: photoSettings, pdf: pdfSettings }), [photoSettings, pdfSettings]);
    const update = useCallback(
      (patch: Partial<S>) => {
        const halves = patch as HalvesPatch<P, D>;
        if ("photo" in halves || "pdf" in halves) {
          if (halves.photo) updatePhoto(halves.photo);
          if (halves.pdf) updatePdf(halves.pdf);
        } else if (isPdf(patch)) {
          updatePdf(patch as Partial<D>);
        } else {
          updatePhoto(patch as Partial<P>);
        }
      },
      [updatePhoto, updatePdf],
    );
    return [settings, update] as const;
  }

  function Main({ settings, onChange, files, paths }: OptionsProps<S>) {
    const noun = pageNoun("file", paths);
    if (noun === "pdf") {
      return <pdf.Main settings={settings.pdf} onChange={(p) => onChange({ pdf: p } as Partial<S>)} files={files} paths={paths} />;
    }
    if (noun === "photo") {
      return <photo.Main settings={settings.photo} onChange={(p) => onChange({ photo: p } as Partial<S>)} files={files} paths={paths} />;
    }
    return <ConvertNote to="any" />;
  }

  function Advanced({ settings, onChange, files, paths }: OptionsProps<S>) {
    const noun = pageNoun("file", paths);
    if (noun === "pdf" && pdf.Advanced) {
      return <pdf.Advanced settings={settings.pdf} onChange={(p) => onChange({ pdf: p } as Partial<S>)} files={files} paths={paths} />;
    }
    if (noun === "photo" && photo.Advanced) {
      return <photo.Advanced settings={settings.photo} onChange={(p) => onChange({ photo: p } as Partial<S>)} files={files} paths={paths} />;
    }
    return null;
  }

  return defineTool<S>({
    noun: "file",
    pick: (current: Noun) => (current === "pdf" ? pdf.pick("pdf") : current === "photo" ? photo.pick("photo") : pickAnyFiles()),
    useSettings,
    Main,
    Advanced,
    showAdvanced: (settings, files) => {
      const noun = readNoun(files);
      if (noun === "pdf") return pdf.Advanced !== undefined && (pdf.showAdvanced?.(settings.pdf, files) ?? true);
      if (noun === "photo") return photo.Advanced !== undefined && (photo.showAdvanced?.(settings.photo, files) ?? true);
      return false;
    },
    toRequest: (settings, files, paths) =>
      pageNoun("file", paths) === "pdf"
        ? pdf.toRequest(settings.pdf, files, paths)
        : photo.toRequest(settings.photo, files, paths),
    presetOptions: (settings) =>
      ({
        photo: photo.presetOptions?.(settings.photo) ?? settings.photo,
        pdf: pdf.presetOptions?.(settings.pdf) ?? settings.pdf,
      }) as Partial<S>,
    blocked: (settings, files, count, tr) => {
      const noun = readNoun(files);
      if (noun === "pdf") return pdf.blocked?.(settings.pdf, files, count, tr) ?? null;
      if (noun === "photo") return photo.blocked?.(settings.photo, files, count, tr) ?? null;
      return null;
    },
    // Each half answers only for its own request.
    leavesUnchanged: (options, file) =>
      (photo.leavesUnchanged?.(options, file) ?? false) || (pdf.leavesUnchanged?.(options, file) ?? false),
    fileNote: (options, file, tr) => photo.fileNote?.(options, file, tr) ?? pdf.fileNote?.(options, file, tr) ?? null,
    summary: (results, tr) => {
      const half = results.length > 0 && nounOf(results[0].inputPath) === "pdf" ? pdf : photo;
      return half.summary?.(results, tr) ?? null;
    },
    comparesSizes: (noun) => (noun === "pdf" ? !pdf.summary : !photo.summary),
  });
}

/** A PDF ke JPG Preset kept how sharp the photos are; Ubah Format Foto's never did. */
const isPdfPreset = (flat: object) => "sharpness" in flat || "dpi" in flat || "mode" in flat;

/** Ubah ke JPG: photos to JPG, or PDF pages to JPG photos. */
export const ubahKeJpg = byNoun(photosToJpg, pdfKeJpg, isPdfPreset);
