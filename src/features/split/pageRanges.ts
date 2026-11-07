/** Pages past this are surely a typo; it also keeps "1-999999999" from filling memory. */
const MOST_PAGES = 10_000;

/**
 * The pages a range like "1-3, 5" names, counted from 1, in order, each once.
 * Empty text names none; null when the text isn't a range.
 */
export function parseRanges(text: string): number[] | null {
  const pages = new Set<number>();
  const parts = text.split(/[,;]/).map((part) => part.trim());
  if (parts.length === 1 && parts[0] === "") return [];
  for (const part of parts) {
    // "1-3", "1 - 3" and "1–3" are all a run; a trailing comma is fine.
    const match = /^(\d+)(?:\s*[-–—]\s*(\d+))?$/.exec(part);
    if (!match) {
      if (part === "") continue;
      return null;
    }
    const from = Number(match[1]);
    const to = match[2] === undefined ? from : Number(match[2]);
    if (from < 1 || to < from || to > MOST_PAGES) return null;
    for (let page = from; page <= to; page++) pages.add(page);
  }
  return [...pages].sort((a, b) => a - b);
}

/** "1-3, 5" for pages 1, 2, 3 and 5, as `pages_suffix` writes them in names. */
export function formatRanges(pages: number[]): string {
  const sorted = [...new Set(pages)].sort((a, b) => a - b);
  const runs: string[] = [];
  for (let i = 0; i < sorted.length; ) {
    let j = i;
    while (j + 1 < sorted.length && sorted[j + 1] === sorted[j] + 1) j++;
    runs.push(j > i ? `${sorted[i]}-${sorted[j]}` : `${sorted[i]}`);
    i = j + 1;
  }
  return runs.join(", ");
}

/** The pages with `page` added, or taken out when it is already there. */
export function togglePage(pages: number[], page: number): number[] {
  return pages.includes(page) ? pages.filter((p) => p !== page) : [...pages, page].sort((a, b) => a - b);
}
