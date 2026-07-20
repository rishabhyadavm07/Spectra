import { useEffect, useRef, useState } from "react";
import type { Environment } from "./types";
import { UNCHANGED_SECRET_SENTINEL } from "./types";

interface Props {
  name: string;
  environment: Environment | null;
  onSave: (name: string, value: string) => Promise<void>;
  onClose: () => void;
  /** Screen position to anchor the popover at — the clicked pill's
   * bounding rect, computed by the caller since only it knows where the
   * pill actually rendered. */
  anchor: { top: number; left: number };
}

/** Small popover shown when a `{{variable}}` pill is clicked — lets the user
 * see and edit that variable's value in the active environment without
 * leaving the request editor, similar to Postman's inline variable editor.
 * Scoped to the single active environment only (see VariablePopoverContext
 * doc comment for why Spectra doesn't need a scope picker here). */
export function VariableEditPopover({
  name,
  environment,
  onSave,
  onClose,
  anchor,
}: Props) {
  const existing = environment?.variables[name];
  const [value, setValue] = useState(existing?.value ?? "");
  const [saving, setSaving] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (
        popoverRef.current &&
        !popoverRef.current.contains(e.target as Node)
      ) {
        onClose();
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [onClose]);

  async function handleSave() {
    setSaving(true);
    try {
      await onSave(name, value);
      onClose();
    } finally {
      setSaving(false);
    }
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      handleSave();
    } else if (e.key === "Escape") {
      onClose();
    }
  }

  const isSecret = existing?.secret ?? false;
  const isUnresolved = !environment || !existing;

  return (
    <div
      ref={popoverRef}
      className="var-edit-popover"
      style={{ top: anchor.top, left: anchor.left }}
      onMouseDown={(e) => e.stopPropagation()}
    >
      <div className="var-edit-popover-name">
        <span className="var-suggest-braces">{"{{ }}"}</span>
        {name}
      </div>

      {isUnresolved ? (
        <div className="var-edit-popover-missing">
          {environment ? (
            <>
              Not defined in <strong>{environment.name}</strong>.
            </>
          ) : (
            "No environment selected."
          )}
        </div>
      ) : (
        <input
          ref={inputRef}
          className="var-edit-popover-input"
          type={isSecret ? "password" : "text"}
          value={value}
          onFocus={(e) => {
            if (isSecret && value === UNCHANGED_SECRET_SENTINEL) {
              setValue("");
              e.target.value = "";
            }
          }}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Enter value"
        />
      )}

      {environment && existing && (
        <div className="var-edit-popover-footer">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button type="button" onClick={handleSave} disabled={saving}>
            {saving ? "Saving…" : "Save"}
          </button>
        </div>
      )}
    </div>
  );
}
