import type { ToolId } from "@/features/home/tools";
import { kompresFoto } from "@/features/compress/ImageOptions";
import { kompresPdf } from "@/features/compress/PdfOptions";
import { ubahFormatFoto } from "@/features/convert/ConvertOptions";
import { ubahUkuranFoto } from "@/features/resize/ResizeOptions";
import { aturHalamanPdf } from "@/features/arrange/ArrangeOptions";
import { gabungPdf } from "@/features/merge/MergeOptions";
import { jpgKePdf } from "@/features/photo-pdf/PhotoPdfOptions";
import { scanDokumen } from "@/features/scan/ScanOptions";
import { pisahPdf } from "@/features/split/SplitOptions";
import { hapusInfoLokasi } from "@/features/clean-info/CleanInfoOptions";
import { beriWatermark } from "@/features/watermark/WatermarkOptions";
import { pdfKeJpg } from "@/features/pdf-to-jpg/PdfToJpgOptions";
import type { ToolDef } from "./toolDef";

// Each Tool keeps its own settings type; the page only passes them through.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type AnyToolDef = ToolDef<any>;

/** The Tools that have a page today. */
export const TOOL_DEFS = {
  "kompres-foto": kompresFoto,
  "kompres-pdf": kompresPdf,
  "ubah-format-foto": ubahFormatFoto,
  "ubah-ukuran-foto": ubahUkuranFoto,
  "gabung-pdf": gabungPdf,
  "pisah-pdf": pisahPdf,
  "atur-halaman-pdf": aturHalamanPdf,
  "jpg-ke-pdf": jpgKePdf,
  "scan-dokumen": scanDokumen,
  "hapus-info-lokasi": hapusInfoLokasi,
  "beri-watermark": beriWatermark,
  "pdf-ke-jpg": pdfKeJpg,
} satisfies Partial<Record<ToolId, AnyToolDef>>;

export type ShippedTool = keyof typeof TOOL_DEFS;

export const isShipped = (id: string): id is ShippedTool => id in TOOL_DEFS;

export const toolDef = (id: ShippedTool): AnyToolDef => TOOL_DEFS[id];
