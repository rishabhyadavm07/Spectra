export interface FormatterMessage {
  body: string;
  format: string;
}

export interface FormatterResult {
  success: boolean;
  result: string;
}

self.onmessage = (e: MessageEvent<FormatterMessage>) => {
  const { body, format } = e.data;

  if (format === "json") {
    try {
      // Parse and format JSON with 2-space indentation
      const parsed = JSON.parse(body);
      const formatted = JSON.stringify(parsed, null, 2);
      self.postMessage({ success: true, result: formatted });
    } catch {
      // If parsing fails, just return the raw string
      self.postMessage({ success: false, result: body });
    }
  } else {
    // For non-JSON formats, return as-is for now
    self.postMessage({ success: true, result: body });
  }
};
