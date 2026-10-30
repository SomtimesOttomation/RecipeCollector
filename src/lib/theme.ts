export type Theme = "system" | "light" | "dark";

const KEY = "lr.theme";

export function loadTheme(): Theme {
  try {
    const t = localStorage.getItem(KEY);
    if (t === "light" || t === "dark" || t === "system") return t;
  } catch {}
  return "system";
}

export function applyTheme(theme: Theme) {
  const root = document.documentElement;
  if (theme === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", theme);
  try { localStorage.setItem(KEY, theme); } catch {}
}

export const isMac = typeof navigator !== "undefined" && /Mac/.test(navigator.userAgent);
