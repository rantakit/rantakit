const UNITS = ["B", "KB", "MB", "GB"];

/** 1536 → "1,5 KB" in Indonesian, "1.5 KB" in English. Uses 1024-based units, like most file managers. */
export function formatBytes(bytes: number, locale: string) {
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(value)} ${UNITS[unit]}`;
}

/** Size change as a signed percentage, e.g. "−75%" or "+3%". */
export function formatChange(before: number, after: number) {
  if (before === 0) return "0%";
  const percent = Math.round(((after - before) / before) * 100);
  if (percent === 0) return "0%";
  return percent < 0 ? `−${-percent}%` : `+${percent}%`;
}

/** The user's today as "2026-11-07", for Result names like `merged 2026-11-07.pdf`. */
export function today() {
  const now = new Date();
  const two = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}-${two(now.getMonth() + 1)}-${two(now.getDate())}`;
}
