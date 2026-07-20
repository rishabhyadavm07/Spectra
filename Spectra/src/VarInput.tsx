import { useRef, useState } from "react";
import type { ChangeEvent, InputHTMLAttributes, KeyboardEvent } from "react";
import { VariableEditPopover } from "./VariableEditPopover";
import { useVariablePopover } from "./VariablePopoverContext";

interface Props extends Omit<
  InputHTMLAttributes<HTMLInputElement>,
  "onChange" | "value"
> {
  value: string;
  onChange: (value: string) => void;
  variableNames: string[];
}

/** Finds an open, unclosed "{{" before the cursor and returns the partial
 * name typed so far, or null if the cursor isn't inside a `{{...` span. */
function findOpenBraceQuery(
  text: string,
  cursor: number,
): { start: number; query: string } | null {
  const upToCursor = text.slice(0, cursor);
  const openIdx = upToCursor.lastIndexOf("{{");
  if (openIdx === -1) return null;
  const closeIdx = upToCursor.lastIndexOf("}}");
  if (closeIdx > openIdx) return null;
  const query = upToCursor.slice(openIdx + 2);
  if (query.includes("{") || query.includes("}")) return null;
  return { start: openIdx, query: query.trim() };
}

interface Token {
  text: string;
  isVariable: boolean;
  varName?: string;
}

/** Splits `text` into plain-text and `{{variable}}` tokens for pill
 * rendering in the overlay layer. */
function tokenize(text: string): Token[] {
  const tokens: Token[] = [];
  const re = /\{\{([^{}]*)\}\}/g;
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = re.exec(text)) !== null) {
    if (match.index > lastIndex)
      tokens.push({
        text: text.slice(lastIndex, match.index),
        isVariable: false,
      });
    tokens.push({ text: match[0], isVariable: true, varName: match[1].trim() });
    lastIndex = match.index + match[0].length;
  }
  if (lastIndex < text.length)
    tokens.push({ text: text.slice(lastIndex), isVariable: false });
  return tokens;
}

/** A text input that offers a `{{` autocomplete dropdown of known variable
 * names (matching Postman/Insomnia's variable suggestion UX), and renders
 * any `{{variable}}` span as a clickable colored pill overlaid on top of a
 * real (invisible-text) input — clicking a pill opens a small popover to
 * view/edit that variable's value in the active environment without
 * leaving the request editor.
 *
 * The pill overlay is skipped for `type="password"` fields: a password
 * input's whole point is masking its content, and an overlay would defeat
 * that by rendering the plaintext underneath the dots.
 */
export function VarInput({
  value,
  onChange,
  variableNames,
  className,
  ...rest
}: Props) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [braceStart, setBraceStart] = useState<number | null>(null);
  const [highlightIndex, setHighlightIndex] = useState(0);
  const [activePopover, setActivePopover] = useState<{
    name: string;
    anchor: { top: number; left: number };
  } | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  const popoverCtx = useVariablePopover();

  const matches = variableNames.filter((v) =>
    v.toLowerCase().includes(query.toLowerCase()),
  );
  const canShowPills = rest.type !== "password" && !!popoverCtx;

  function updateSuggestion(text: string, cursor: number) {
    const found = findOpenBraceQuery(text, cursor);
    if (found) {
      setOpen(true);
      setQuery(found.query);
      setBraceStart(found.start);
      setHighlightIndex(0);
    } else {
      setOpen(false);
    }
  }

  function handleChange(e: ChangeEvent<HTMLInputElement>) {
    const next = e.target.value;
    onChange(next);
    updateSuggestion(next, e.target.selectionStart ?? next.length);
    // The browser adjusts scrollLeft as part of this same input event when
    // typing pushes the caret past the visible edge — read it next frame so
    // the overlay picks up the post-adjustment value, not the stale one.
    requestAnimationFrame(syncOverlayScroll);
  }

  function applySuggestion(name: string) {
    if (braceStart === null || !inputRef.current) return;
    const cursor = inputRef.current.selectionStart ?? value.length;
    const before = value.slice(0, braceStart);
    const after = value.slice(cursor);
    const next = `${before}{{${name}}}${after}`;
    onChange(next);
    setOpen(false);
    requestAnimationFrame(() => {
      const pos = before.length + name.length + 4;
      inputRef.current?.setSelectionRange(pos, pos);
      inputRef.current?.focus();
    });
  }

  function handleKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (open && matches.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setHighlightIndex((i) => (i + 1) % matches.length);
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setHighlightIndex((i) => (i - 1 + matches.length) % matches.length);
        return;
      }
      if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        applySuggestion(matches[highlightIndex]);
        return;
      }
      if (e.key === "Escape") {
        setOpen(false);
        return;
      }
    }
    rest.onKeyDown?.(e);
  }

  function handlePillClick(e: React.MouseEvent, varName: string) {
    e.preventDefault();
    e.stopPropagation();
    if (!wrapRef.current) return;
    const wrapRect = wrapRef.current.getBoundingClientRect();
    const pillRect = (e.target as HTMLElement).getBoundingClientRect();
    setActivePopover({
      name: varName,
      anchor: {
        top: pillRect.bottom - wrapRect.top + 4,
        left: pillRect.left - wrapRect.left,
      },
    });
  }

  function syncOverlayScroll() {
    // A single-line <input> scrolls its text horizontally once it overflows
    // the visible width (no scrollbar shown, but `scrollLeft` still moves as
    // you type past the edge) — without this, the pill overlay stays put
    // while the real input's text scrolls underneath it, so typing past the
    // input's width permanently desyncs the two layers.
    if (inputRef.current && wrapRef.current) {
      const overlay = wrapRef.current.querySelector<HTMLDivElement>(
        ".var-input-pill-layer",
      );
      if (overlay) overlay.scrollLeft = inputRef.current.scrollLeft;
    }
  }

  return (
    <div className="var-input-wrap" ref={wrapRef}>
      <input
        {...rest}
        ref={inputRef}
        className={
          canShowPills
            ? `${className ?? ""} var-input-overlay-input`.trim()
            : className
        }
        value={value}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        onScroll={syncOverlayScroll}
        onSelect={syncOverlayScroll}
        onBlur={(e) => {
          window.setTimeout(() => setOpen(false), 120);
          rest.onBlur?.(e);
        }}
      />
      {canShowPills && (
        <div className="var-input-pill-layer" aria-hidden="true">
          {tokenize(value).map((token, i) =>
            token.isVariable ? (
              <span
                key={i}
                className="var-pill"
                style={{ pointerEvents: "auto" }}
                onMouseDown={(e) => handlePillClick(e, token.varName!)}
                title={
                  popoverCtx?.activeEnvironment?.variables[token.varName!]?.secret
                    ? "Secret Variable"
                    : popoverCtx?.activeEnvironment?.variables[token.varName!]?.value ?? "Unknown Variable"
                }
              >
                {token.text}
              </span>
            ) : (
              <span key={i}>{token.text}</span>
            ),
          )}
        </div>
      )}
      {open && matches.length > 0 && (
        <div className="var-suggest-menu">
          {matches.map((name, i) => (
            <button
              key={name}
              type="button"
              className={
                i === highlightIndex
                  ? "var-suggest-item active"
                  : "var-suggest-item"
              }
              onMouseDown={(e) => {
                e.preventDefault();
                applySuggestion(name);
              }}
              onMouseEnter={() => setHighlightIndex(i)}
            >
              <span className="var-suggest-braces">{"{{ }}"}</span>
              {name}
            </button>
          ))}
        </div>
      )}
      {open && matches.length === 0 && query !== "" && (
        <div className="var-suggest-menu">
          <div className="var-suggest-empty">No matching variables</div>
        </div>
      )}
      {activePopover && popoverCtx && (
        <VariableEditPopover
          name={activePopover.name}
          environment={popoverCtx.activeEnvironment}
          anchor={activePopover.anchor}
          onSave={popoverCtx.onUpdateVariable}
          onClose={() => setActivePopover(null)}
        />
      )}
    </div>
  );
}
