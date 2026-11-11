import {
  useEffect,
  useId,
  useRef,
  useState,
  type ComponentType,
  type KeyboardEvent as ReactKeyboardEvent,
  type PointerEvent as ReactPointerEvent,
} from "react";
import {
  ArrowLeft,
  Bin,
  Camera,
  ChevronLeft,
  ChevronRight,
  CircleAlert,
  Image,
  LoaderCircle,
  Palette,
  Plus,
  RotateCw,
  Scan,
  Sparkles,
  type IconProps,
} from "@keyline-icons/react";
import { Button as AriaButton } from "react-aria-components/Button";
import { GridList, GridListItem } from "react-aria-components/GridList";
import { ProgressBar } from "react-aria-components/ProgressBar";
import { DropIndicator, useDragAndDrop } from "react-aria-components/useDragAndDrop";
import { Button } from "@/components/ui/Button";
import { Dialog, Heading } from "@/components/ui/Dialog";
import { Menu, MenuItem } from "@/components/ui/Menu";
import { Popover } from "@/components/ui/Popover";
import { Sheet } from "@/components/ui/Sheet";
import { ToggleButton } from "@/components/ui/ToggleButton";
import { ToggleButtonGroup } from "@/components/ui/ToggleButtonGroup";
import { pickImages } from "@/features/compress/imageApi";
import { CropHost, NoMarginOption, PaperField, PdfNote } from "@/features/photo-pdf/PhotoPdfOptions";
import { openCropEditor, turnPhoto, usePhotoEdits, type EditState } from "@/features/photo-pdf/photoEdits";
import { useThumbnail } from "@/features/tool/fileInfo";
import { moveBy, reorder } from "@/features/tool/order";
import type { ReviewProps } from "@/features/tool/toolDef";
import { MoreOptions } from "@/features/tool/ToolPage";
import { useT } from "@/i18n";
import { isDesktopApp } from "@/lib/platform";
import {
  askCameraPermission,
  cameraPermission,
  discardPhotos,
  takePhotos,
  type CameraPermission,
} from "./cameraApi";
import { forgetPages, usePreparedPages, type PageState } from "./preparedPages";
import type { ScanSettings } from "./ScanOptions";

/** How long "Urungkan" stays after a page is deleted. */
const UNDO_MS = 5000;
/** How far a finger moves sideways to turn to the next page, in px. */
const SWIPE_PX = 48;

/** A deleted page, kept until "Urungkan" goes away. */
interface Removed {
  path: string;
  /** Where it was, counted from 0. */
  at: number;
}

/**
 * Scan Dokumen's pages once the camera closes (D3): one page at a time,
 * prepared in the background, with crop, turn and delete for that page,
 * the order in a strip below, and "Simpan PDF" to pack them.
 */
export function ScanReview({ paths, settings, onChange, onBack, onAdd, onReorder, onStart, working }: ReviewProps<ScanSettings>) {
  const { t, count } = useT();
  const headingId = useId();
  const [index, setIndex] = useState(0);
  const at = Math.min(index, paths.length - 1);
  const path = paths[at];
  const pages = usePreparedPages(paths, settings.look === "clean", at);
  const edits = usePhotoEdits(paths);
  const [sheet, setSheet] = useState<"add" | "save" | null>(null);
  /** "Simpan" was pressed while pages were still being prepared. */
  const [waiting, setWaiting] = useState(false);
  const busy = paths.filter((p) => pages[p].status !== "ready").length;

  const go = (to: number) => setIndex(Math.max(0, Math.min(paths.length - 1, to)));

  // Each move is said aloud; the count makes a repeated message new.
  const [said, setSaid] = useState({ text: "", count: 0 });
  /** The pages in a new order, still showing the same page. */
  const reorderTo = (next: string[]) => {
    if (next === paths) return;
    onReorder(next);
    setIndex(next.indexOf(path));
  };
  const move = (moving: string, by: -1 | 1) => {
    const from = paths.indexOf(moving);
    const next = moveBy(paths, from, by);
    if (next === paths) return;
    onReorder(next);
    if (moving === path) setIndex(from + by);
    setSaid((last) => ({ text: t("files.moved", { position: from + by + 1 }), count: last.count + 1 }));
  };

  // Delete with "Urungkan": the photo is only thrown away once that goes.
  const [removed, setRemoved] = useState<Removed | null>(null);
  const pending = useRef<Removed | null>(null);
  const timer = useRef<number | undefined>(undefined);
  const throwAway = () => {
    window.clearTimeout(timer.current);
    if (pending.current) {
      discardPhotos([pending.current.path]);
      forgetPages([pending.current.path]);
      pending.current = null;
    }
    setRemoved(null);
  };
  const throwAwayLater = useRef(throwAway);
  throwAwayLater.current = throwAway;
  // Leaving the page throws a deleted page away at once.
  useEffect(() => () => throwAwayLater.current(), []);

  const remove = () => {
    if (paths.length < 2) return;
    throwAway();
    const gone = { path, at };
    pending.current = gone;
    setRemoved(gone);
    onReorder(paths.filter((p) => p !== path));
    timer.current = window.setTimeout(() => throwAwayLater.current(), UNDO_MS);
  };
  const undo = () => {
    const gone = pending.current;
    if (!gone) return;
    pending.current = null;
    window.clearTimeout(timer.current);
    setRemoved(null);
    const next = [...paths];
    next.splice(Math.min(gone.at, next.length), 0, gone.path);
    onReorder(next);
    setIndex(gone.at);
  };

  const addPages = (added: string[]) => {
    if (added.length === 0) return;
    const first = paths.length;
    onAdd(added);
    setIndex(first);
  };

  const save = () => {
    if (busy > 0) {
      setWaiting(true);
      return;
    }
    setSheet(null);
    onStart();
  };
  // The PDF is saved by itself once the last page is ready.
  useEffect(() => {
    if (!waiting || busy > 0) return;
    setWaiting(false);
    setSheet(null);
    onStart();
    // `onStart` is the current render's.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [waiting, busy]);

  const swipeFrom = useRef<number | null>(null);
  const onPointerDown = (event: ReactPointerEvent) => {
    swipeFrom.current = event.pointerType === "mouse" ? null : event.clientX;
  };
  const onPointerUp = (event: ReactPointerEvent) => {
    const from = swipeFrom.current;
    swipeFrom.current = null;
    if (from === null || Math.abs(event.clientX - from) < SWIPE_PX) return;
    go(event.clientX < from ? at + 1 : at - 1);
  };

  const edit = edits[path];
  const page = pages[path];

  return (
    <div className="relative flex flex-1 flex-col max-md:h-dvh md:min-h-0">
      <header className="flex h-14 shrink-0 items-center gap-2 px-2 md:px-4">
        <Button variant="icon" aria-label={t("tool.back")} onPress={onBack} className={isDesktopApp ? "md:hidden" : undefined}>
          <ArrowLeft aria-hidden className="size-5" />
        </Button>
        <h1 id={headingId} className="truncate ps-1 font-display text-section">
          {t("page.label", { page: at + 1, pages: paths.length })}
        </h1>
      </header>

      <div inert={working !== null} className={`flex min-h-0 flex-1 flex-col ${working ? "opacity-45" : ""}`}>
        <ToggleButtonGroup
          aria-label={t("scan.look")}
          selectionMode="single"
          disallowEmptySelection
          selectedKeys={[settings.look]}
          onSelectionChange={(keys) => {
            const look = [...keys][0];
            if (look === "clean" || look === "original") onChange({ look });
          }}
          className="mx-4 mb-3 grid shrink-0 grid-cols-2 md:mx-auto md:w-96"
        >
          <ToggleButton id="clean" className="h-10 w-full">
            <Sparkles aria-hidden className="size-5" /> {t("scan.look.clean")}
          </ToggleButton>
          <ToggleButton id="original" className="h-10 w-full">
            <Palette aria-hidden className="size-5" /> {t("scan.look.original")}
          </ToggleButton>
        </ToggleButtonGroup>

        <section
          aria-labelledby={headingId}
          onPointerDown={onPointerDown}
          onPointerUp={onPointerUp}
          onPointerCancel={() => (swipeFrom.current = null)}
          className="relative flex min-h-0 flex-1 touch-pan-y flex-col bg-canvas"
        >
          <div className="flex min-h-0 flex-1 items-center justify-center gap-4 overflow-hidden py-4">
            <Slot path={paths[at - 1]} state={pages[paths[at - 1]]} />
            <Slot path={path} state={page} position={at + 1} main />
            <Slot path={paths[at + 1]} state={pages[paths[at + 1]]} />
          </div>
          <p role="status" className="flex shrink-0 items-center justify-center gap-1.5 px-4 pb-3 text-small text-text-muted">
            <PageStatus edit={edit} page={page} />
          </p>
          {at > 0 && (
            <Button
              variant="secondary"
              aria-label={t("scan.prev")}
              onPress={() => go(at - 1)}
              className="absolute start-2 top-1/2 size-11 -translate-y-8 rounded-full px-0"
            >
              <ChevronLeft aria-hidden className="size-5" />
            </Button>
          )}
          {at < paths.length - 1 && (
            <Button
              variant="secondary"
              aria-label={t("scan.next")}
              onPress={() => go(at + 1)}
              className="absolute end-2 top-1/2 size-11 -translate-y-8 rounded-full px-0"
            >
              <ChevronRight aria-hidden className="size-5" />
            </Button>
          )}
        </section>

        <div role="toolbar" aria-label={t("scan.actions")} className="grid shrink-0 grid-cols-3 gap-2 px-4 pt-2 md:mx-auto md:w-96">
          <ToolbarButton label={t("scan.crop")} icon={Scan} isDisabled={edit.source === "finding"} onPress={() => openCropEditor(path)} />
          <ToolbarButton label={t("scan.turn")} icon={RotateCw} isDisabled={edit.source === "finding"} onPress={() => turnPhoto(path)} />
          <ToolbarButton label={t("scan.remove")} icon={Bin} isDisabled={paths.length < 2} onPress={remove} />
        </div>

        <PageStrip
          paths={paths}
          current={path}
          pages={pages}
          onGo={(to) => go(paths.indexOf(to))}
          onReorder={reorderTo}
          onMove={move}
          onAdd={() => setSheet("add")}
        />
      </div>

      <div className="flex shrink-0 flex-col gap-2 px-4 pt-2 pb-5 md:mx-auto md:w-96">
        {working ?? <Button onPress={() => setSheet("save")}>{t("main.scan-dokumen")}</Button>}
      </div>

      {removed && (
        <div
          role="status"
          className="absolute inset-x-4 bottom-[88px] z-10 flex min-h-13 items-center justify-between gap-3 rounded-[14px] bg-text py-1 ps-4 pe-1 text-surface shadow-lg md:mx-auto md:w-96"
        >
          <span className="text-label">{t("scan.removed", { position: removed.at + 1 })}</span>
          <AriaButton
            onPress={undo}
            className="h-11 rounded-[10px] px-3.5 text-label font-bold text-brand-soft outline-none data-[focus-visible]:outline-2 data-[focus-visible]:outline-surface"
          >
            {t("arrange.undo")}
          </AriaButton>
        </div>
      )}

      <div role="status" className="sr-only" key={said.count}>
        {said.text}
      </div>

      <CropHost paths={paths} />

      <AddSheet isOpen={sheet === "add"} onClose={() => setSheet(null)} onAdd={addPages} />

      <Sheet
        isOpen={sheet === "save"}
        isDismissable={!waiting}
        onOpenChange={(open) => {
          if (open) return;
          setSheet(null);
          setWaiting(false);
        }}
      >
        <Dialog className="p-4 pb-5 md:p-6">
          <SheetHandle />
          <Heading slot="title" className="my-0 font-display text-dialog">
            {t("main.scan-dokumen")}
          </Heading>
          <div className="mt-4 flex flex-col gap-[22px]">
            <PaperField paper={settings.paper} onChange={(paper) => onChange({ paper })} />
            <PdfNote files={count("pages", paths.length)} paper={settings.paper} />
            {settings.paper !== "photo" && (
              <MoreOptions tool="scan-dokumen">
                <NoMarginOption settings={settings} onChange={onChange} files={[]} paths={paths} />
              </MoreOptions>
            )}
            {waiting ? (
              <>
                <ProgressBar
                  aria-label={count("scan.save.waiting", busy)}
                  value={paths.length - busy}
                  maxValue={paths.length}
                  className="flex flex-col gap-2 rounded-[14px] bg-brand-soft p-3 text-brand-strong"
                >
                  {({ percentage }) => (
                    <>
                      <span className="text-label font-semibold">{count("scan.save.waiting", busy)}</span>
                      <span className="h-1.5 overflow-hidden rounded-full bg-track">
                        <span className="block h-full rounded-full bg-brand-strong transition-[width]" style={{ width: `${percentage}%` }} />
                      </span>
                      <span className="text-small">{t("scan.save.waiting.body")}</span>
                    </>
                  )}
                </ProgressBar>
                <Button variant="secondary" onPress={() => setWaiting(false)}>
                  {t("common.cancel")}
                </Button>
              </>
            ) : (
              <Button onPress={save}>{t("common.save")}</Button>
            )}
          </div>
        </Dialog>
      </Sheet>
    </div>
  );
}

/** "Dipotong otomatis", "Potongan sudah diatur", or "Menyiapkan halaman…". */
function PageStatus({ edit, page }: { edit: EditState; page: PageState }) {
  const { t } = useT();
  if (page.status !== "ready") {
    return (
      <>
        <LoaderCircle aria-hidden className="size-4 shrink-0 motion-safe:animate-spin" />
        {t("scan.preparing")}
      </>
    );
  }
  if (edit.source === "notFound") {
    return (
      <span className="flex items-center gap-1.5 font-semibold text-warn">
        <CircleAlert aria-hidden className="size-4 shrink-0" />
        {t("crop.notFound")}
      </span>
    );
  }
  return (
    <>
      <Scan aria-hidden className="size-4 shrink-0" />
      {edit.source === "found" ? t("crop.auto") : edit.edit.corners ? t("crop.set") : t("crop.whole.used")}
    </>
  );
}

interface SlotProps {
  path: string | undefined;
  state: PageState | undefined;
  /** The page in the middle; the others peek in at the sides. */
  main?: boolean;
  position?: number;
}

/** One page of the carousel: the page as it goes in the PDF, or the photo dimmed while it is prepared. */
function Slot({ path, state, main = false, position }: SlotProps) {
  const { t } = useT();
  const photo = useThumbnail(path ?? null, main ? 800 : 400);
  const width = main ? "w-[64%] md:w-[min(64%,26rem)]" : "w-[52%] md:w-[min(52%,20rem)]";
  if (!path || !state) return <div aria-hidden className={`h-full shrink-0 ${width}`} />;
  const ready = state.status === "ready";
  const src = (ready && state.url) || photo;
  return (
    <div aria-hidden={!main} className={`relative flex h-full shrink-0 items-center justify-center ${width} ${main ? "" : "opacity-45"}`}>
      {src && (
        <img
          src={src}
          alt={main && position ? t("scan.thumb", { position }) : ""}
          draggable={false}
          className={`max-h-full max-w-full rounded-sm object-contain shadow-[0_6px_24px_rgb(17_24_39/0.18)] transition-opacity ${
            ready ? "" : "opacity-40"
          }`}
        />
      )}
      {main && !ready && <LoaderCircle aria-hidden className="absolute size-8 text-brand-strong motion-safe:animate-spin" />}
    </div>
  );
}

interface PageStripProps {
  paths: string[];
  current: string;
  pages: Record<string, PageState>;
  onGo: (path: string) => void;
  onReorder: (paths: string[]) => void;
  onMove: (path: string, by: -1 | 1) => void;
  onAdd: () => void;
}

/**
 * Every page small, in order: tap one to open it, drag one to move it, or
 * tap the open one for "Pindah ke kiri" and "Pindah ke kanan". Alt with an
 * arrow key moves the focused page, as in Atur Halaman PDF.
 */
function PageStrip({ paths, current, pages, onGo, onReorder, onMove, onAdd }: PageStripProps) {
  const { t } = useT();
  const items = useRef(new Map<string, HTMLElement>());
  const anchor = useRef<HTMLElement | null>(null);
  const [moving, setMoving] = useState(false);

  const { dragAndDropHooks } = useDragAndDrop({
    getItems: (keys) => [...keys].map((key) => ({ "text/plain": t("scan.thumb", { position: paths.indexOf(String(key)) + 1 }) })),
    onReorder: (event) => {
      if (event.target.dropPosition === "on") return;
      onReorder(reorder(paths, new Set([...event.keys].map(String)), String(event.target.key), event.target.dropPosition));
    },
    renderDropIndicator: (target) => (
      <DropIndicator
        target={target}
        className={({ isDropTarget }) => `w-0.5 self-stretch rounded-full ${isDropTarget ? "bg-brand-strong" : "bg-transparent"}`}
      />
    ),
  });

  const onKeyDown = (event: ReactKeyboardEvent) => {
    if (!event.altKey) return;
    const by = event.key === "ArrowLeft" ? -1 : event.key === "ArrowRight" ? 1 : 0;
    const key = (event.target as HTMLElement).closest("[data-key]")?.getAttribute("data-key");
    if (by === 0 || !key) return;
    event.preventDefault();
    event.stopPropagation();
    onMove(key, by);
  };

  // The open page stays in view.
  useEffect(() => {
    items.current.get(current)?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [current, paths.length]);

  const at = paths.indexOf(current);
  return (
    <div className="flex shrink-0 items-start gap-1.5 overflow-x-auto border-t border-border px-3 pt-2 pb-1 md:justify-center">
      <div onKeyDownCapture={onKeyDown} className="contents">
        <GridList
          aria-label={t("scan.order")}
          layout="grid"
          items={paths.map((id) => ({ id }))}
          dragAndDropHooks={dragAndDropHooks}
          onAction={(key) => {
            const pressed = String(key);
            if (pressed !== current) return onGo(pressed);
            anchor.current = items.current.get(pressed) ?? null;
            setMoving(true);
          }}
          className="flex gap-1.5 outline-none"
        >
          {({ id }) => {
            const position = paths.indexOf(id) + 1;
            return (
              <GridListItem
                id={id}
                textValue={t("scan.thumb", { position })}
                ref={(element: HTMLDivElement | null) => {
                  if (element) items.current.set(id, element);
                  else items.current.delete(id);
                }}
                className={({ isDragging }) => `shrink-0 rounded-[10px] outline-none ${isDragging ? "opacity-50" : ""}`}
              >
                {({ isFocusVisible }) => (
                  <Thumb path={id} state={pages[id]} position={position} isCurrent={id === current} isFocusVisible={isFocusVisible} />
                )}
              </GridListItem>
            );
          }}
        </GridList>
      </div>
      <Button
        variant="secondary"
        aria-label={t("scan.add")}
        onPress={onAdd}
        className="mt-1 ms-0.5 h-[61px] w-[52px] rounded-[10px] border-2 border-dashed px-0 text-brand-strong"
      >
        <Plus aria-hidden className="size-5" />
      </Button>

      <Popover triggerRef={anchor} isOpen={moving} onOpenChange={setMoving} placement="top">
        <Menu
          aria-label={t("scan.thumb", { position: at + 1 })}
          autoFocus="first"
          disabledKeys={[...(at === 0 ? ["left"] : []), ...(at === paths.length - 1 ? ["right"] : [])]}
          onAction={(key) => {
            onMove(current, key === "left" ? -1 : 1);
            setMoving(false);
          }}
        >
          <MenuItem id="left">{t("scan.moveLeft")}</MenuItem>
          <MenuItem id="right">{t("scan.moveRight")}</MenuItem>
        </Menu>
      </Popover>
    </div>
  );
}

interface ThumbProps {
  path: string;
  state: PageState;
  position: number;
  isCurrent: boolean;
  isFocusVisible: boolean;
}

function Thumb({ path, state, position, isCurrent, isFocusVisible }: ThumbProps) {
  const photo = useThumbnail(path, 96);
  const ready = state.status === "ready";
  const src = (ready && state.url) || photo;
  return (
    <div
      className={`flex w-[52px] flex-col items-center gap-1 rounded-[10px] p-1 ${isCurrent ? "bg-brand-soft text-brand-strong" : "text-text-muted"} ${
        isFocusVisible ? "outline-3 outline-solid outline-brand-strong" : ""
      }`}
    >
      <span className={`flex h-[57px] w-10 items-center justify-center overflow-hidden rounded-sm ${isCurrent ? "ring-2 ring-brand-strong" : ""}`}>
        {src && <img src={src} alt="" draggable={false} className={`max-h-full max-w-full object-contain ${ready ? "" : "opacity-40"}`} />}
      </span>
      <span className="text-[12px] leading-4 font-semibold">{position}</span>
    </div>
  );
}

/** A phone toolbar button: its icon over a short label. */
function ToolbarButton({
  label,
  icon: Icon,
  isDisabled,
  onPress,
}: {
  label: string;
  icon: ComponentType<IconProps>;
  isDisabled?: boolean;
  onPress: () => void;
}) {
  return (
    <Button
      variant="icon"
      isDisabled={isDisabled}
      onPress={onPress}
      className={`h-auto min-h-15 w-full flex-col gap-0.5 rounded-[14px] px-1 py-2 ${isDisabled ? "" : "text-text"}`}
    >
      <Icon aria-hidden className="size-6" />
      <span className="text-label leading-tight font-semibold">{label}</span>
    </Button>
  );
}

function SheetHandle() {
  return <span aria-hidden className="mx-auto mb-3 block h-1 w-10 rounded-full bg-border-strong md:hidden" />;
}

/** "Tambah halaman": the camera again, or photos from Gallery. */
function AddSheet({ isOpen, onClose, onAdd }: { isOpen: boolean; onClose: () => void; onAdd: (paths: string[]) => void }) {
  const { t } = useT();
  const [camera, setCamera] = useState<CameraPermission | null>(null);
  useEffect(() => {
    if (isOpen) cameraPermission().then(setCamera);
  }, [isOpen]);

  const fromCamera = async () => {
    onClose();
    if (camera === "ask" && !(await askCameraPermission())) {
      setCamera(await cameraPermission());
      return;
    }
    onAdd(await takePhotos());
  };
  const fromGallery = async () => {
    onClose();
    onAdd(await pickImages());
  };

  return (
    <Sheet isOpen={isOpen} isDismissable onOpenChange={(open) => !open && onClose()}>
      <Dialog className="p-4 pb-6 md:p-6">
        <SheetHandle />
        <Heading slot="title" className="my-0 mb-3 font-display text-dialog">
          {t("scan.add")}
        </Heading>
        <div className="flex flex-col gap-2">
          {camera !== "refused" && (
            <AddOption icon={Camera} label={t("scan.more")} description={t("scan.add.camera")} onPress={fromCamera} />
          )}
          <AddOption icon={Image} label={t("scan.gallery")} description={t("scan.add.gallery")} onPress={fromGallery} />
        </div>
      </Dialog>
    </Sheet>
  );
}

function AddOption({
  icon: Icon,
  label,
  description,
  onPress,
}: {
  icon: ComponentType<IconProps>;
  label: string;
  description: string;
  onPress: () => void;
}) {
  return (
    <AriaButton
      onPress={onPress}
      className="flex min-h-16 items-center gap-3.5 rounded-[14px] border border-border bg-surface px-3 text-start outline-none hover:bg-badge-bg data-[focus-visible]:outline-3 data-[focus-visible]:outline-brand-strong"
    >
      <span className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-brand-soft text-brand-strong">
        <Icon aria-hidden className="size-[22px]" />
      </span>
      <span className="flex flex-col">
        <span className="text-label font-semibold">{label}</span>
        <span className="text-small text-text-muted">{description}</span>
      </span>
    </AriaButton>
  );
}
