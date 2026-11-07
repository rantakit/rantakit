/** Puts `moving` before or after `target`, keeping the order among the files moved. */
export function reorder(paths: string[], moving: Set<string>, target: string, position: "before" | "after") {
  if (moving.has(target) || !paths.includes(target)) return paths;
  const kept = paths.filter((path) => !moving.has(path));
  const moved = paths.filter((path) => moving.has(path));
  const at = kept.indexOf(target) + (position === "after" ? 1 : 0);
  return [...kept.slice(0, at), ...moved, ...kept.slice(at)];
}

/** Moves the file at `from` by one place up (-1) or down (1). */
export function moveBy(paths: string[], from: number, by: -1 | 1) {
  const to = from + by;
  if (from < 0 || to < 0 || to >= paths.length) return paths;
  const next = [...paths];
  next.splice(from, 1);
  next.splice(to, 0, paths[from]);
  return next;
}
