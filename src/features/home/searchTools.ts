import { translator } from "@/i18n";
import type { ToolId, ToolInfo } from "./tools";

/** Extra words people type for each Tool, in either language. Never shown. */
const KEYWORDS: Record<ToolId, string> = {
  "kompres-foto": "compress reduce shrink photo image picture gambar perkecil",
  "kompres-pdf": "compress reduce shrink perkecil",
  "ubah-format-foto": "convert konversi format jpg jpeg png heic heif webp ke to",
  "ubah-ukuran-foto": "resize scale dimensions pixels perbesar perkecil kecilkan ukuran",
  "kompres-dokumen": "compress office document dokumen word excel powerpoint ppt doc docx xls xlsx pptx",
  "kompres-audio": "compress sound voice note suara",
  "ubah-format-audio": "convert konversi sound suara mp3",
  "kompres-video": "compress",
  "word-ke-pdf": "convert konversi docx doc",
  "excel-ke-pdf": "convert konversi xlsx xls spreadsheet",
  "jpg-ke-pdf": "convert konversi photo image foto scan",
  "scan-dokumen": "camera kamera pindai foto dokumen scanner",
  "gabung-pdf": "merge combine join satukan gabungkan",
  "pisah-pdf": "split separate extract",
  "atur-halaman-pdf": "arrange reorder rotate delete pages putar hapus",
  "pdf-ke-jpg": "convert konversi image gambar foto",
  "beri-watermark": "watermark stamp tanda air cap",
  "hapus-info-lokasi": "remove location gps exif metadata privacy privasi",
};

const LANGUAGES = [translator("id"), translator("en")];

/** Lower case, no accents, split into words. */
function words(text: string) {
  return text
    .toLowerCase()
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .split(/[^\p{L}\p{N}]+/u)
    .filter(Boolean);
}

const index = new Map<ToolId, string[]>();

function wordsOf(id: ToolId) {
  let found = index.get(id);
  if (!found) {
    const text = LANGUAGES.flatMap(({ t }) => [t(`tool.${id}`), t(`tool.${id}.description`)]);
    found = words([...text, KEYWORDS[id]].join(" "));
    index.set(id, found);
  }
  return found;
}

/**
 * The Tools whose names, descriptions, or keywords, in Indonesian or English,
 * contain every word of `query`. The last word may be unfinished, so it only
 * has to start a word: "kom" finds Kompres Foto while the user is typing.
 */
export function searchTools(tools: ToolInfo[], query: string): ToolInfo[] {
  const wanted = words(query);
  if (wanted.length === 0) return tools;
  const last = wanted.length - 1;
  return tools.filter((tool) => {
    const have = wordsOf(tool.id);
    return wanted.every((word, i) => (i === last ? have.some((w) => w.startsWith(word)) : have.includes(word)));
  });
}
