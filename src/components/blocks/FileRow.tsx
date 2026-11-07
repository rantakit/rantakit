import type { ComponentType, ReactNode } from "react";
import { Check, CircleX, X, type IconProps } from "@keyline-icons/react";
import { ProgressBar } from "react-aria-components";
import { Button } from "@/components/ui/Button";

export type FileRowStatus = "ready" | "waiting" | "working" | "done" | "skipped" | "failed";

interface FileRowProps {
  name: string;
  /** Full path, shown on hover. */
  title?: string;
  /** Stroke icon for the file's kind, shown in the thumbnail tile. */
  icon: ComponentType<IconProps>;
  /** A small picture of the file, shown in the tile instead of the icon once loaded. */
  thumbnail?: string | null;
  /** "page" shows the thumbnail whole, as a white page with a thin edge; "photo" fills the tile. */
  thumbnailKind?: "photo" | "page";
  status: FileRowStatus;
  /** The line under the name: a size, "Menunggu", "Selesai · 196 KB", or a message. */
  note: ReactNode;
  /** 0–1, while working. */
  fraction?: number;
  /** Ready rows only: removes the file from the list. */
  onRemove?: () => void;
  removeLabel?: string;
  /** Shown at the end of the row instead of the remove button, e.g. "Tampilkan di folder". */
  action?: ReactNode;
}

const NOTE_ICON: Partial<Record<FileRowStatus, [ComponentType<IconProps>, string]>> = {
  done: [Check, "text-success font-semibold"],
  skipped: [Check, "text-text-muted font-semibold"],
  failed: [CircleX, "text-danger font-semibold"],
};

/** A file in the Files step; each state has its own icon or text, never color alone. */
export function FileRow({
  name,
  title,
  icon: Icon,
  thumbnail,
  thumbnailKind = "photo",
  status,
  note,
  fraction = 0,
  onRemove,
  removeLabel,
  action,
}: FileRowProps) {
  const [NoteIcon, noteStyle] = NOTE_ICON[status] ?? [null, "text-text-muted"];

  return (
    <li className="flex items-center gap-3.5 py-3 ps-3 pe-2">
      <FileTile icon={Icon} thumbnail={thumbnail} thumbnailKind={thumbnailKind} />
      <span className="flex min-w-0 flex-1 flex-col gap-1">
        <span className="truncate text-label" title={title}>
          {name}
        </span>
        <span className={`flex items-center gap-1.5 text-small ${noteStyle}`}>
          {NoteIcon && <NoteIcon aria-hidden className="size-4 shrink-0" />}
          <span className="min-w-0 truncate">{note}</span>
        </span>
        {status === "working" && (
          <ProgressBar aria-label={name} value={fraction * 100} className="h-1.5 overflow-hidden rounded-full bg-badge-bg">
            {({ percentage }) => <div className="h-full rounded-full bg-brand-strong" style={{ width: `${percentage}%` }} />}
          </ProgressBar>
        )}
      </span>
      {action ?? (status === "ready" && onRemove ? (
        <Button variant="icon" aria-label={removeLabel} onPress={onRemove}>
          <X aria-hidden className="size-5" />
        </Button>
      ) : (
        <span className="w-11 shrink-0" />
      ))}
    </li>
  );
}

interface FileTileProps {
  icon: ComponentType<IconProps>;
  thumbnail?: string | null;
  thumbnailKind?: "photo" | "page";
}

/** The square at the start of a file row: the photo, the first page, or the kind's icon. */
export function FileTile({ icon: Icon, thumbnail, thumbnailKind = "photo" }: FileTileProps) {
  return (
    <span
      aria-hidden
      className="flex size-12 shrink-0 items-center justify-center overflow-hidden rounded-[10px] bg-badge-bg text-text-muted"
    >
      {thumbnail ? (
        thumbnailKind === "page" ? (
          <img src={thumbnail} alt="" className="max-h-10 max-w-10 rounded-[2px] border border-page-border bg-page" />
        ) : (
          <img src={thumbnail} alt="" className="size-full object-cover" />
        )
      ) : (
        <Icon className="size-6" />
      )}
    </span>
  );
}
