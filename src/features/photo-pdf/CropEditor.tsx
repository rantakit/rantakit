import { useEffect, useId, useLayoutEffect, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import { Image, RotateCw, X } from "@keyline-icons/react";
import { Dialog, Heading } from "react-aria-components/Dialog";
import { Modal, ModalOverlay } from "react-aria-components/Modal";
import { Button } from "@/components/ui/Button";
import { useThumbnail } from "@/features/tool/fileInfo";
import { useT } from "@/i18n";
import { WHOLE, type Corners, type PhotoEdit } from "./photoEdits";

interface CropEditorProps {
  path: string;
  name: string;
  /** The photo's place in the list, from 1, and how many there are: "Foto 2 dari 3". */
  position: number;
  count: number;
  edit: PhotoEdit;
  onDone: (edit: PhotoEdit) => void;
  onCancel: () => void;
}

/** The photo is drawn this wide in the editor's own units; its height follows. */
const UNITS = 1000;
const CORNERS = ["topLeft", "topRight", "bottomRight", "bottomLeft"] as const;
/** On-screen sizes, in px. */
const HANDLE = 10;
const HANDLE_HIT = 24;
const LENS = 50;
const LENS_ZOOM = 3;

const clamp = (value: number) => Math.min(1, Math.max(0, value));

/** A screen direction as a direction on the photo, which is turned `turns` quarters clockwise. */
function unturned(dx: number, dy: number, turns: number) {
  let [x, y] = [dx, dy];
  for (let i = 0; i < turns % 4; i++) [x, y] = [y, -x];
  return [x, y] as const;
}

/**
 * "Potong & luruskan" (D1): the whole screen, the photo with its four
 * corners to drag onto the paper's edges. Touch shows a magnifier at the
 * corner being dragged, since the finger covers it. Each corner is also a
 * button the arrow keys move. "Batal", Escape, and Android's back keep the
 * crop as it was.
 */
export function CropEditor({ path, name, position, count, edit, onDone, onCancel }: CropEditorProps) {
  const { t } = useT();
  const photo = useThumbnail(path, 1024);
  const [size, setSize] = useState<{ w: number; h: number } | null>(null);
  const [corners, setCorners] = useState<Corners>(edit.corners ?? WHOLE);
  const [turns, setTurns] = useState(((edit.turns % 4) + 4) % 4);
  const [drag, setDrag] = useState<{ corner: number; touch: boolean } | null>(null);
  const [focused, setFocused] = useState<number | null>(null);
  const svgRef = useRef<SVGSVGElement>(null);
  const photoRef = useRef<SVGGElement>(null);
  /** Editor units per on-screen px. */
  const [unit, setUnit] = useState(1);
  const lensId = useId();

  useEffect(() => {
    if (!photo) return;
    const img = new window.Image();
    img.onload = () => setSize({ w: img.naturalWidth, h: img.naturalHeight });
    img.src = photo;
  }, [photo]);

  const W = UNITS;
  const H = size ? (UNITS * size.h) / size.w : UNITS;
  const sideways = turns % 2 === 1;
  const [viewW, viewH] = sideways ? [H, W] : [W, H];
  const turn = [``, `translate(${H} 0) rotate(90)`, `translate(${W} ${H}) rotate(180)`, `translate(0 ${W}) rotate(270)`][turns];

  useLayoutEffect(() => {
    const svg = svgRef.current;
    if (!svg) return;
    const measure = () => {
      const box = svg.getBoundingClientRect();
      if (box.width > 0 && box.height > 0) setUnit(Math.max(viewW / box.width, viewH / box.height));
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(svg);
    return () => observer.disconnect();
  }, [viewW, viewH, size]);

  const moveTo = (corner: number, x: number, y: number) =>
    setCorners((all) => all.map((point, i) => (i === corner ? [clamp(x), clamp(y)] : point)) as Corners);

  const onPointerDown = (corner: number) => (event: PointerEvent<SVGGElement>) => {
    // Keeps the drag on this corner when the pointer leaves it.
    try {
      event.currentTarget.setPointerCapture(event.pointerId);
    } catch {
      // A pointer the browser no longer tracks: the drag still works while over the corner.
    }
    setDrag({ corner, touch: event.pointerType !== "mouse" });
  };
  const onPointerMove = (event: PointerEvent<SVGGElement>) => {
    const matrix = photoRef.current?.getScreenCTM();
    if (!drag || !matrix) return;
    const point = new DOMPoint(event.clientX, event.clientY).matrixTransform(matrix.inverse());
    moveTo(drag.corner, point.x / W, point.y / H);
  };
  const onKeyDown = (corner: number) => (event: KeyboardEvent<SVGGElement>) => {
    const arrows: Record<string, [number, number]> = {
      ArrowLeft: [-1, 0],
      ArrowRight: [1, 0],
      ArrowUp: [0, -1],
      ArrowDown: [0, 1],
    };
    const arrow = arrows[event.key];
    if (!arrow) return;
    event.preventDefault();
    const step = event.shiftKey ? 0.05 : 0.01;
    const [dx, dy] = unturned(arrow[0], arrow[1], turns);
    moveTo(corner, corners[corner][0] + dx * step, corners[corner][1] + dy * step);
  };

  const at = corners.map(([x, y]) => [x * W, y * H] as const);
  const outline = at.map(([x, y]) => `${x},${y}`).join(" ");
  const lens = drag?.touch ? lensFor(at[drag.corner], W, H, unit) : null;
  const subtitle = count > 1 ? t("crop.position", { position, count }) : null;

  return (
    <ModalOverlay isOpen isDismissable={false} onOpenChange={(open) => !open && onCancel()} className="fixed inset-0 z-50 bg-bg pt-[env(safe-area-inset-top)] pb-[env(safe-area-inset-bottom)]">
      <Modal className="flex h-full w-full flex-col text-text">
        <Dialog aria-label={t("crop.title")} className="flex h-full flex-col outline-none">
          <header className="flex shrink-0 items-center justify-between gap-2 border-b border-border px-3 py-3 md:gap-4 md:px-6 md:py-4">
            <Button variant="secondary" size="row" onPress={onCancel}>
              <X aria-hidden className="size-5" /> {t("crop.cancel")}
            </Button>
            <div className="flex min-w-0 flex-col items-center text-center">
              <Heading slot="title" className="whitespace-nowrap font-display text-section font-semibold md:text-wordmark">
                {t("crop.title")}
              </Heading>
              <span className="max-w-full truncate text-small text-text-muted">
                <span>{subtitle}</span>
                <span className="hidden md:inline">
                  {subtitle && " · "}
                  {name}
                </span>
              </span>
            </div>
            <Button size="row" className="md:min-w-30" onPress={() => onDone({ corners, turns })}>
              {t("crop.done")}
            </Button>
          </header>

          <div className="flex min-h-0 flex-1 items-center justify-center bg-editor p-4 md:p-7">
            {photo && size ? (
              <svg
                ref={svgRef}
                viewBox={`0 0 ${viewW} ${viewH}`}
                className="h-full w-full touch-none select-none"
                role="group"
                aria-label={t("crop.photo", { name })}
              >
                <g ref={photoRef} transform={turn}>
                  <image href={photo} width={W} height={H} preserveAspectRatio="none" />
                  <path
                    d={`M0 0H${W}V${H}H0Z M${at.map(([x, y]) => `${x} ${y}`).join("L")}Z`}
                    fill="#000"
                    fillOpacity={0.5}
                    fillRule="evenodd"
                  />
                  <polygon points={outline} fill="none" stroke="#2dd4bf" strokeWidth={2.5 * unit} />
                  {at.map(([x, y], i) => (
                    <g
                      key={CORNERS[i]}
                      role="button"
                      tabIndex={0}
                      aria-label={t(`crop.corner.${CORNERS[i]}`)}
                      className="cursor-grab outline-none"
                      onPointerDown={onPointerDown(i)}
                      onPointerMove={onPointerMove}
                      onPointerUp={() => setDrag(null)}
                      onPointerCancel={() => setDrag(null)}
                      onKeyDown={onKeyDown(i)}
                      onFocus={() => setFocused(i)}
                      onBlur={() => setFocused(null)}
                    >
                      <circle cx={x} cy={y} r={HANDLE_HIT * unit} fill="transparent" />
                      {focused === i && (
                        <circle cx={x} cy={y} r={(HANDLE + 6) * unit} fill="none" stroke="#fff" strokeWidth={3 * unit} />
                      )}
                      {drag?.corner === i && drag.touch ? (
                        <circle cx={x} cy={y} r={16 * unit} fill="#2dd4bf" fillOpacity={0.35} stroke="#fff" strokeWidth={3 * unit} />
                      ) : (
                        <circle cx={x} cy={y} r={HANDLE * unit} fill="#fff" stroke="#0f766e" strokeWidth={3 * unit} />
                      )}
                    </g>
                  ))}
                  {lens && drag && (
                    <g aria-hidden pointerEvents="none">
                      <clipPath id={lensId}>
                        <circle cx={lens.x} cy={lens.y} r={lens.r} />
                      </clipPath>
                      <g clipPath={`url(#${lensId})`}>
                        <rect x={lens.x - lens.r} y={lens.y - lens.r} width={lens.r * 2} height={lens.r * 2} fill="#000" />
                        <g
                          transform={`translate(${lens.x} ${lens.y}) scale(${LENS_ZOOM}) translate(${-at[drag.corner][0]} ${-at[drag.corner][1]})`}
                        >
                          <image href={photo} width={W} height={H} preserveAspectRatio="none" />
                          <polygon points={outline} fill="none" stroke="#2dd4bf" strokeWidth={(2.5 * unit) / LENS_ZOOM} />
                        </g>
                      </g>
                      <circle cx={lens.x} cy={lens.y} r={lens.r} fill="none" stroke="#fff" strokeWidth={3 * unit} />
                      <path
                        d={`M${lens.x - 6 * unit} ${lens.y}H${lens.x + 6 * unit}M${lens.x} ${lens.y - 6 * unit}V${lens.y + 6 * unit}`}
                        stroke="#2dd4bf"
                        strokeWidth={2 * unit}
                        strokeLinecap="round"
                      />
                    </g>
                  )}
                </g>
              </svg>
            ) : (
              <span className="text-small text-white/80">{t("crop.loading")}</span>
            )}
          </div>

          <footer className="flex shrink-0 flex-col gap-3 border-t border-border px-4 py-3 md:flex-row md:items-center md:justify-between md:px-6 md:py-4">
            <p className="text-center text-small text-text-muted md:text-start md:text-label md:font-normal">
              <span className="md:hidden">{t("crop.hint.short")}</span>
              <span className="hidden md:inline">{t("crop.hint")}</span>
            </p>
            <div className="flex gap-2">
              <Button variant="secondary" size="row" className="flex-1 md:flex-none" onPress={() => setTurns((n) => (n + 1) % 4)}>
                <RotateCw aria-hidden className="size-5" /> {t("crop.rotate")}
              </Button>
              <Button variant="secondary" size="row" className="flex-1 md:flex-none" onPress={() => setCorners(WHOLE)}>
                <Image aria-hidden className="size-5" /> {t("crop.whole")}
              </Button>
            </div>
          </footer>
        </Dialog>
      </Modal>
    </ModalOverlay>
  );
}

/** Where the magnifier goes: off the corner toward the photo's middle, clear of the finger. */
function lensFor([x, y]: readonly [number, number], W: number, H: number, unit: number) {
  const r = LENS * unit;
  const [dx, dy] = [W / 2 - x, H / 2 - y];
  const length = Math.hypot(dx, dy) || 1;
  const away = (LENS + 40) * unit;
  return {
    x: Math.min(W - r, Math.max(r, x + (dx / length) * away)),
    y: Math.min(H - r, Math.max(r, y + (dy / length) * away)),
    r,
  };
}
