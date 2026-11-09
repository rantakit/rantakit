import { useEffect, useRef, useSyncExternalStore } from "react";

/**
 * What Undo in the title bar menu undoes: the page on screen offers it
 * with `useUndoTarget` while it has something to undo. Only Arrange PDF Pages
 * does today.
 */
let target: { undo: () => void } | null = null;
const listeners = new Set<() => void>();

function setTarget(next: typeof target) {
  target = next;
  listeners.forEach((listener) => listener());
}

/** Offer `undo` to the menu while `canUndo` is true. */
export function useUndoTarget(canUndo: boolean, undo: () => void) {
  const undoRef = useRef(undo);
  undoRef.current = undo;

  useEffect(() => {
    if (!canUndo) return;
    const offered = { undo: () => undoRef.current() };
    setTarget(offered);
    return () => {
      if (target === offered) setTarget(null);
    };
  }, [canUndo]);
}

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};

/** Whether anything on screen can be undone now. */
export function useCanUndo() {
  return useSyncExternalStore(subscribe, () => target !== null);
}

/** Undoes the last change on screen, if there is one. */
export function undoLast() {
  target?.undo();
}
