/** One page of the Result: which page of the Original, and how far it is turned. */
export interface PlacedPage {
  /** Stays with the page as it moves, for selection and dragging. */
  key: string;
  /** Page of the Original, from 0. */
  page: number;
  /** Quarter turns clockwise, 0–3. */
  turns: number;
}

/** The Original as it is: every page in order, unturned. */
export function originalPages(count: number): PlacedPage[] {
  return Array.from({ length: count }, (_, page) => ({ key: `p${page}`, page, turns: 0 }));
}

/** Turns the pages in `keys` a quarter clockwise (1) or anticlockwise (-1). */
export function rotate(pages: PlacedPage[], keys: Set<string>, by: 1 | -1): PlacedPage[] {
  return pages.map((p) => (keys.has(p.key) ? { ...p, turns: (p.turns + by + 4) % 4 } : p));
}

export function remove(pages: PlacedPage[], keys: Set<string>): PlacedPage[] {
  return pages.filter((p) => !keys.has(p.key));
}

/** Whether saving would make anything different from the Original. */
export function isChanged(pages: PlacedPage[], count: number) {
  return pages.length !== count || pages.some((p, i) => p.page !== i || p.turns !== 0);
}
