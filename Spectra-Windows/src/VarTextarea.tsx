import { useRef, useState } from "react";
import type { ChangeEvent, KeyboardEvent, TextareaHTMLAttributes } from "react";
import { VariableEditPopover } from "./VariableEditPopover";
import { useVariablePopover } from "./VariablePopoverContext";

interface Props extends Omit<
  TextareaHTMLAttributes<HTMLTextAreaElement>,
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
  // A trailing newline needs an extra soft-break token to reflect it in the
  // overlay layer the same way a <textarea> reserves a trailing blank line.
  if (text.endsWith("\n")) tokens.push({ text: "​", isVariable: false });
  return tokens;
}

/** Textarea counterpart to VarInput — same `{{` autocomplete behavior plus
 * the same clickable variable-pill overlay, for multi-line body content.
 * The suggestion menu anchors below the textarea as a whole rather than
 * tracking exact caret pixel position (a full caret-coordinate mirror would
 * need a hidden-div text-measuring trick; not worth the complexity for a
 * response/request body editor). */
export function VarTextarea({
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
  const areaRef = useRef<HTMLTextAreaElement>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  const popoverCtx = useVariablePopover();

  const matches = variableNames.filter((v) =>
    v.toLowerCase().includes(query.toLowerCase()),
  );
  const canShowPills = !!popoverCtx;

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

  function handleChange(e: ChangeEvent<HTMLTextAreaElement>) {
    const next = e.target.value;
    onChange(next);
    updateSuggestion(next, e.target.selectionStart ?? next.length);
  }

  function applySuggestion(name: string) {
    if (braceStart === null || !areaRef.current) return;
    const cursor = areaRef.current.selectionStart ?? value.length;
    const before = value.slice(0, braceStart);
    const after = value.slice(cursor);
    const next = `${before}{{${name}}}${after}`;
    onChange(next);
    setOpen(false);
    requestAnimationFrame(() => {
      const pos = before.length + name.length + 4;
      areaRef.current?.setSelectionRange(pos, pos);
      areaRef.current?.focus();
    });
  }

  function handleKeyDown(e: KeyboardEvent<HTMLTextAreaElement>) {
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

  function handleScroll() {
    // Keep the overlay's scroll position glued to the real textarea's, so
    // pills stay aligned with their underlying text while scrolling.
    if (areaRef.current && wrapRef.current) {
      const overlay = wrapRef.current.querySelector<HTMLDivElement>(
        ".var-input-pill-layer",
      );
      if (overlay) {
        overlay.scrollTop = areaRef.current.scrollTop;
        overlay.scrollLeft = areaRef.current.scrollLeft;
      }
    }
  }

  return (
    <div className="var-input-wrap var-textarea-wrap" ref={wrapRef}>
      <textarea
        {...rest}
        ref={areaRef}
        className={
          canShowPills
            ? `${className ?? ""} var-input-overlay-input`.trim()
            : className
        }
        value={value}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        onScroll={handleScroll}
        onBlur={(e) => {
          window.setTimeout(() => setOpen(false), 120);
          rest.onBlur?.(e);
        }}
      />
      {canShowPills && (
        <div
          className="var-input-pill-layer var-textarea-pill-layer"
          aria-hidden="true"
        >
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
        <div className="var-suggest-menu var-suggest-menu-textarea">
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
        <div className="var-suggest-menu var-suggest-menu-textarea">
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
