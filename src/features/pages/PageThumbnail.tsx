import { useEffect, useRef, useState, type RefObject } from "react";
import { useT } from "@/i18n";
import { usePage } from "./pageThumbnails";

interface PageThumbnailProps {
  path: string;
  /** Counting from 0. */
  page: number;
  /** How many pages the PDF has, for "Halaman 3 dari 12". */
  pages: number;
  /** Longest side to draw, in px; about twice the width shown. */
  size?: number;
  selected?: boolean;
  /** Clockwise, in degrees: 0, 90, 180, or 270. */
  rotation?: number;
  className?: string;
}

/**
 * One PDF page as it looks (X1). Until it is drawn, and when it can't be, it
 * is a numbered page. The page keeps its own white in both themes and is
 * never inverted; a screen reader hears "Halaman 3 dari 12, diputar 90°, dipilih".
 */
export function PageThumbnail({ path, page, pages, size = 240, selected, rotation = 0, className = "" }: PageThumbnailProps) {
  const { t } = useT();
  const box = useRef<HTMLDivElement>(null);
  const visible = useSeen(box);
  const url = usePage(path, page, size, visible);
  const turned = ((rotation % 360) + 360) % 360;
  const label = [
    t("page.label", { page: page + 1, pages }),
    turned ? t("page.rotated", { degrees: turned }) : null,
    selected ? t("page.selected") : null,
  ]
    .filter(Boolean)
    .join(", ");
  // A quarter turn swaps width and height, so the page shrinks to stay inside its box.
  const transform = turned ? `rotate(${turned}deg)${turned % 180 ? " scale(0.707)" : ""}` : undefined;

  return (
    <div
      ref={box}
      role="img"
      aria-label={label}
      className={`relative flex aspect-[1/1.414] items-center justify-center rounded-md ${
        selected ? "outline-3 outline-offset-3 outline-brand-strong" : ""
      } ${className}`}
    >
      <div className="flex size-full items-center justify-center transition-transform" style={{ transform }}>
        {url ? (
          <img
            src={url}
            alt=""
            draggable={false}
            className="max-h-full max-w-full rounded-[3px] border border-page-border bg-page"
          />
        ) : (
          <span
            aria-hidden
            className="flex size-full items-center justify-center rounded-[3px] border border-page-border bg-page text-section text-page-number"
          >
            {page + 1}
          </span>
        )}
      </div>
      {turned > 0 && (
        <span
          aria-hidden
          className="absolute end-1.5 top-1.5 rounded-full bg-brand-strong px-2 text-small font-semibold text-on-primary"
        >
          {turned}°
        </span>
      )}
    </div>
  );
}

/** True once the element has come near the screen; it stays true. */
function useSeen(ref: RefObject<HTMLElement | null>) {
  const [seen, setSeen] = useState(false);
  useEffect(() => {
    const element = ref.current;
    if (!element || seen) return;
    if (typeof IntersectionObserver === "undefined") {
      setSeen(true);
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setSeen(true);
          observer.disconnect();
        }
      },
      { rootMargin: "200px" },
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, [ref, seen]);
  return seen;
}
