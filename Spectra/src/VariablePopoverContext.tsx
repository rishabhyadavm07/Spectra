import { createContext, useContext } from "react";
import type { Environment } from "./types";

export interface VariablePopoverContextValue {
  /** The workspace's currently active environment, or null if none is
   * selected — the popover edits variables here and nowhere else (Spectra
   * has no Postman-style Collection/Global/Vault scoping). */
  activeEnvironment: Environment | null;
  /** Persists a single variable's value to the active environment,
   * preserving every other variable and its secret flag untouched. */
  onUpdateVariable: (name: string, value: string) => Promise<void>;
}

export const VariablePopoverContext =
  createContext<VariablePopoverContextValue | null>(null);

/** Returns null (rather than throwing) when no provider is mounted, so
 * VarInput/VarTextarea can render their plain autocomplete-only behavior
 * anywhere they're used outside the main request editor (tests, storybook,
 * or a future usage this session didn't anticipate) instead of crashing. */
export function useVariablePopover(): VariablePopoverContextValue | null {
  return useContext(VariablePopoverContext);
}
