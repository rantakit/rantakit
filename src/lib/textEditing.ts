/**
 * Cut, Copy, Paste, and Select all in the title bar menu, for the text
 * field the user was typing in. Opening the menu moves focus away from the
 * field, so the last one focused is remembered until focus goes anywhere
 * other than the menu.
 */

type TextField = HTMLInputElement | HTMLTextAreaElement;

const TEXT_INPUTS = new Set(["text", "search", "url", "email", "tel", "number", "password"]);

/** Marks the menu's trigger; focus on it or on the menu keeps the field remembered. */
export const MENU_ATTRIBUTE = "data-app-menu";

let lastField: TextField | null = null;

function isTextField(el: EventTarget | null): el is TextField {
  if (el instanceof HTMLTextAreaElement) return true;
  return el instanceof HTMLInputElement && TEXT_INPUTS.has(el.type);
}

if (typeof document !== "undefined") {
  document.addEventListener("focusin", (event) => {
    const el = event.target;
    if (isTextField(el)) lastField = el;
    else if (!(el instanceof Element && el.closest(`[${MENU_ATTRIBUTE}], [role="menu"]`))) lastField = null;
  });
}

/** What the clipboard items can act on now, or null when no text field was in use. */
export function editableField() {
  const field = lastField;
  if (!field || !field.isConnected || field.disabled) return null;
  const hasSelection = field.selectionStart !== field.selectionEnd;
  return { field, canChange: !field.readOnly, hasSelection, hasText: field.value.length > 0 };
}

export type TextAction = "cut" | "copy" | "paste" | "select_all";

/**
 * Runs `action` on the remembered field. The menu gives focus back to its
 * trigger as it closes, so the field takes it back afterwards.
 */
export async function editText(action: TextAction) {
  const field = editableField()?.field;
  if (!field) return;
  takeFocusBack(field);
  field.focus();
  switch (action) {
    case "cut":
    case "copy":
      document.execCommand(action);
      break;
    case "select_all":
      field.select();
      break;
    case "paste": {
      const text = await navigator.clipboard.readText().catch(() => "");
      if (text) {
        field.focus();
        document.execCommand("insertText", false, text);
      }
      break;
    }
  }
}

/** When the closing menu focuses its trigger, focus `field` again; gives up after a second. */
function takeFocusBack(field: TextField) {
  const onFocus = (event: FocusEvent) => {
    if (event.target instanceof Element && event.target.closest(`[${MENU_ATTRIBUTE}]`)) {
      stop();
      field.focus();
    }
  };
  const timer = setTimeout(() => stop(), 1000);
  const stop = () => {
    clearTimeout(timer);
    document.removeEventListener("focusin", onFocus);
  };
  document.addEventListener("focusin", onFocus);
}
