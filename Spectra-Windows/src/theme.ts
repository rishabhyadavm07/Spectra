export function applyTheme(theme: string) {
  const root = document.documentElement;
  root.classList.remove("dark", "crimson");

  if (theme === "system") {
    if (
      window.matchMedia &&
      window.matchMedia("(prefers-color-scheme: dark)").matches
    ) {
      root.classList.add("dark");
    }
  } else if (theme === "dark") {
    root.classList.add("dark");
  } else if (theme === "crimson") {
    root.classList.add("crimson");
  }
}

// Global listener for system theme changes
if (window.matchMedia) {
  window
    .matchMedia("(prefers-color-scheme: dark)")
    .addEventListener("change", () => {
      // Only re-apply if the current setting is "system"
      import("@tauri-apps/api/core")
        .then(({ invoke }) => {
          invoke("get_settings")
            .then((settings: any) => {
              if (settings.theme === "system") {
                applyTheme("system");
              }
            })
            .catch(console.error);
        })
        .catch(console.error);
    });
}
