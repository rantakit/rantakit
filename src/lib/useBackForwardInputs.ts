import { useEffect, useRef } from "react";

interface Inputs {
  /** The mouse's side buttons are Back and Forward. */
  mouse: boolean;
  /** Escape is Back, such as on a Tool page. */
  escape: boolean;
}

/**
 * Back and Forward from Escape and the mouse's side buttons, each while it is
 * turned on. `go` may change every render; the latest one answers.
 */
export function useBackForwardInputs(go: (delta: -1 | 1) => void, { mouse, escape }: Inputs) {
  // Keep the latest callback without re-subscribing on every render.
  const goRef = useRef(go);
  goRef.current = go;

  useEffect(() => {
    if (!escape) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !e.defaultPrevented) goRef.current(-1);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [escape]);

  // Cancelling the side buttons also keeps the webview from trying a navigation of its own.
  useEffect(() => {
    if (!mouse) return;
    const isSideButton = (e: MouseEvent) => e.button === 3 || e.button === 4;
    const onMouseDown = (e: MouseEvent) => {
      if (isSideButton(e)) e.preventDefault();
    };
    const onMouseUp = (e: MouseEvent) => {
      if (!isSideButton(e)) return;
      e.preventDefault();
      goRef.current(e.button === 3 ? -1 : 1);
    };
    window.addEventListener("mousedown", onMouseDown);
    window.addEventListener("mouseup", onMouseUp);
    return () => {
      window.removeEventListener("mousedown", onMouseDown);
      window.removeEventListener("mouseup", onMouseUp);
    };
  }, [mouse]);
}
