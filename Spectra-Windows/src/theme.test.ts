import { describe, it, expect, beforeEach } from 'vitest';
import { applyTheme } from './theme';

describe('Theme utility', () => {
  beforeEach(() => {
    // Reset classes before each test
    document.documentElement.className = '';
  });

  it('applies dark class when dark mode is selected', () => {
    applyTheme('dark');
    expect(document.documentElement.classList.contains('dark')).toBe(true);
    expect(document.documentElement.classList.contains('crimson')).toBe(false);
  });

  it('applies crimson class when crimson mode is selected', () => {
    applyTheme('crimson');
    expect(document.documentElement.classList.contains('crimson')).toBe(true);
    expect(document.documentElement.classList.contains('dark')).toBe(false);
  });

  it('removes classes when light mode is selected', () => {
    applyTheme('dark');
    applyTheme('light');
    expect(document.documentElement.classList.contains('dark')).toBe(false);
    expect(document.documentElement.classList.contains('crimson')).toBe(false);
  });
});
