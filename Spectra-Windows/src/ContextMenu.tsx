import { useEffect, useRef } from "react";

export interface ContextMenuItem {
  label: string;
  onClick: () => void;
  shortcut?: string;
  danger?: boolean;
  disabled?: boolean;
  /** Renders a divider line above this item. */
  separatorBefore?: boolean;
}

interface Props {
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
}

/** Generic right-click context menu, positioned at a fixed viewport point.
 * Closes on click-outside, Escape, or after an item is clicked. */
export function ContextMenu({ x, y, items, onClose }: Props) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    }
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("mousedown", handleClick);
    document.addEventListener("keydown", handleKey);
    return () => {
      document.removeEventListener("mousedown", handleClick);
      document.removeEventListener("keydown", handleKey);
    };
  }, [onClose]);

  // Keep the menu on-screen if it would overflow the right/bottom edge.
  const style: React.CSSProperties = { left: x, top: y };
  if (typeof window !== "undefined") {
    const estWidth = 220;
    const estHeight = items.length * 30 + 16;
    if (x + estWidth > window.innerWidth)
      style.left = window.innerWidth - estWidth - 8;
    if (y + estHeight > window.innerHeight)
      style.top = window.innerHeight - estHeight - 8;
  }

  return (
    <div className="context-menu" style={style} ref={ref}>
      {items.map((item, i) => (
        <div key={i}>
          {item.separatorBefore && <div className="context-menu-divider" />}
          <button
            className={
              item.danger ? "context-menu-item danger" : "context-menu-item"
            }
            disabled={item.disabled}
            onClick={() => {
              item.onClick();
              onClose();
            }}
          >
            <span>{item.label}</span>
            {item.shortcut && (
              <span className="context-menu-shortcut">{item.shortcut}</span>
            )}
          </button>
        </div>
      ))}
    </div>
  );
}
