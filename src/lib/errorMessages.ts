import type { MessageKey, Translator } from "@/i18n";
import { formatBytes } from "./format";

/** Every code the backend sends; each has an "error.<code>" string in both languages. */
type ErrorCode =
  | "password-protected"
  | "damaged"
  | "unsupported-format"
  | "size-limit-unreachable"
  | "not-enough-space"
  | "cannot-write"
  | "original-missing"
  | "no-such-pages"
  | "cancelled"
  | "unchanged"
  | "history-from-newer-version"
  | "unexpected";

const CODES: ErrorCode[] = [
  "password-protected",
  "damaged",
  "unsupported-format",
  "size-limit-unreachable",
  "not-enough-space",
  "cannot-write",
  "original-missing",
  "no-such-pages",
  "cancelled",
  "unchanged",
  "history-from-newer-version",
  "unexpected",
];

interface ErrorContext {
  /** The formats the Tool opens, "JPG, PNG": added after "can't open this kind of file". */
  accepted?: string;
  /** A follow-up button offers smaller images: the Size limit message asks about it. */
  canShrink?: boolean;
}

/**
 * What to tell the user about an error from the backend: `{ code, best? }`, or
 * anything else. Cancelled files get no message here; their row says "Dibatalkan".
 */
export function errorMessage({ t, locale }: Translator, error: unknown, context: ErrorContext = {}): string {
  const { code, best } = (error ?? {}) as { code?: unknown; best?: unknown };
  const known = CODES.find((c) => c === code) ?? "unexpected";
  switch (known) {
    case "size-limit-unreachable": {
      const message =
        typeof best === "number"
          ? t("error.size-limit-unreachable", { best: formatBytes(best, locale) })
          : t("error.size-limit-unreachable.unknown");
      return context.canShrink ? `${message} ${t("error.size-limit-unreachable.shrink")}` : message;
    }
    case "unsupported-format":
      return context.accepted
        ? `${t("error.unsupported-format")} ${t("error.accepted", { formats: context.accepted })}`
        : t("error.unsupported-format");
    case "cancelled":
      return t("task.file.cancelled");
    default:
      return t(`error.${known}` satisfies MessageKey);
  }
}
