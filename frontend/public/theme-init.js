(() => {
  const storageKey = "suno-documentation-theme";
  let savedTheme = null;
  try {
    savedTheme = window.localStorage.getItem(storageKey);
  } catch {
    // A blocked storage backend must not prevent the local app from starting.
  }
  const theme = savedTheme === "light" || savedTheme === "dark"
    ? savedTheme
    : window.matchMedia?.("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  document.documentElement.dataset.theme = theme;
  document.documentElement.style.colorScheme = theme;
  document.querySelector('meta[name="theme-color"]')
    ?.setAttribute("content", theme === "dark" ? "#111310" : "#f4f2ed");
})();
