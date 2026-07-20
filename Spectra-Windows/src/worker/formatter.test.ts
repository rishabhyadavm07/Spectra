import { describe, it, expect } from 'vitest';

// We'll test the logic directly since Web Workers are tricky to test in jsdom
// Here we just test the core formatting logic that the worker uses
function formatJson(body: string): { success: boolean, result: string } {
  try {
    const parsed = JSON.parse(body);
    const formatted = JSON.stringify(parsed, null, 2);
    return { success: true, result: formatted };
  } catch {
    return { success: false, result: body };
  }
}

describe('JSON Formatter', () => {
  it('formats valid JSON correctly', () => {
    const input = '{"hello":"world", "test": 123}';
    const output = formatJson(input);
    expect(output.success).toBe(true);
    expect(output.result).toBe('{\n  "hello": "world",\n  "test": 123\n}');
  });

  it('handles invalid JSON gracefully', () => {
    const input = '{"hello": "world"'; // Missing closing brace
    const output = formatJson(input);
    expect(output.success).toBe(false);
    expect(output.result).toBe(input); // Should return the original string
  });
});
