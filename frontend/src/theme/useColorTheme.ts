import { useEffect, useState } from "react";

export type ColorMode = "dark" | "light";
export type ThemeId = "better-auth" | "zinc" | "ember" | "arctic";
export type RadiusPreset = "default" | "small" | "medium" | "large";
export type ThemeColors = Record<string, string>;
export type ThemeDefinition = {
  id: ThemeId;
  name: string;
  description: string;
  dark: ThemeColors;
  light: ThemeColors;
};

export type ThemeControls = {
  mode: ColorMode;
  radius: RadiusPreset;
  setMode: (mode: ColorMode) => void;
  setRadius: (radius: RadiusPreset) => void;
  setThemeId: (themeId: ThemeId) => void;
  themeId: ThemeId;
  themes: ThemeDefinition[];
  toggleMode: () => void;
};

const THEME_STORAGE_KEY = "accounts-repo-color-theme";
const MODE_STORAGE_KEY = "accounts-repo-color-mode";
const RADIUS_STORAGE_KEY = "accounts-repo-border-radius";
const DEFAULT_THEME_ID: ThemeId = "better-auth";
const DEFAULT_RADIUS: RadiusPreset = "default";

const RADIUS_PRESETS: Record<RadiusPreset, Record<string, string>> = {
  default: { "--radius-sm": "0.05rem", "--radius-md": "0.125rem", "--radius-lg": "0.25rem" },
  small: { "--radius-sm": "0.125rem", "--radius-md": "0.25rem", "--radius-lg": "0.375rem" },
  medium: { "--radius-sm": "0.25rem", "--radius-md": "0.5rem", "--radius-lg": "0.75rem" },
  large: { "--radius-sm": "0.375rem", "--radius-md": "0.75rem", "--radius-lg": "1.25rem" },
};

const THEMES: ThemeDefinition[] = [
  {
    id: "better-auth",
    name: "Better Auth",
    description: "The default Better Hub theme: near-black, zinc surfaces, sparse accent color.",
    dark: {
      "--background": "#030304",
      "--foreground": "#fafafa",
      "--card": "#111113",
      "--card-foreground": "#fafafa",
      "--primary": "#e4e4e7",
      "--primary-foreground": "#09090b",
      "--secondary": "#1a1a1e",
      "--secondary-foreground": "#fafafa",
      "--muted": "#1a1a1e",
      "--muted-foreground": "#a1a1aa",
      "--accent": "#1a1a1e",
      "--accent-foreground": "#fafafa",
      "--border": "#27272a",
      "--input": "#27272a",
      "--ring": "#3f3f46",
      "--destructive": "oklch(0.704 0.191 22.216)",
      "--success": "oklch(0.627 0.194 149.214)",
      "--warning": "oklch(0.769 0.188 70.08)",
      "--scrollbar-thumb": "#3f3f46",
      "--scrollbar-thumb-hover": "#52525b",
      "--link": "#58a6ff",
      "--info": "#58a6ff",
      "--code-bg": "#09090b",
      "--code-block-bg": "#111113",
      "--inline-code-bg": "rgba(63, 63, 70, 0.5)",
      "--selection-bg": "oklch(0.7 0.1 285 / 20%)",
      "--table-row-alt": "#111113",
    },
    light: {
      "--background": "#ffffff",
      "--foreground": "#18181b",
      "--card": "#f9f9f9",
      "--card-foreground": "#18181b",
      "--primary": "#18181b",
      "--primary-foreground": "#ffffff",
      "--secondary": "#f4f4f5",
      "--secondary-foreground": "#18181b",
      "--muted": "#f4f4f5",
      "--muted-foreground": "#71717a",
      "--accent": "#f0f0f1",
      "--accent-foreground": "#18181b",
      "--border": "#e4e4e7",
      "--input": "#e4e4e7",
      "--ring": "#a1a1aa",
      "--destructive": "#dc2626",
      "--success": "#16a34a",
      "--warning": "#ca8a04",
      "--scrollbar-thumb": "#d4d4d8",
      "--scrollbar-thumb-hover": "#a1a1aa",
      "--link": "#2563eb",
      "--info": "#2563eb",
      "--code-bg": "#ffffff",
      "--code-block-bg": "#f4f4f5",
      "--inline-code-bg": "rgba(0, 0, 0, 0.05)",
      "--selection-bg": "rgba(59, 130, 246, 0.18)",
      "--table-row-alt": "#f9f9f9",
    },
  },
  {
    id: "zinc",
    name: "Zinc",
    description: "Lifted zinc tones from Better Hub's built-in theme set.",
    dark: {
      "--background": "#101012",
      "--foreground": "#ececef",
      "--card": "#181819",
      "--card-foreground": "#ececef",
      "--primary": "#dddde2",
      "--primary-foreground": "#101012",
      "--secondary": "#1f1f22",
      "--secondary-foreground": "#ececef",
      "--muted": "#1f1f22",
      "--muted-foreground": "#8b8b98",
      "--accent": "#1f1f22",
      "--accent-foreground": "#ececef",
      "--border": "#2a2a2e",
      "--input": "#2a2a2e",
      "--ring": "#52525b",
      "--destructive": "oklch(0.704 0.191 22.216)",
      "--success": "oklch(0.627 0.194 149.214)",
      "--warning": "oklch(0.769 0.188 70.08)",
      "--scrollbar-thumb": "#343438",
      "--scrollbar-thumb-hover": "#44444c",
      "--link": "#58a6ff",
      "--info": "#58a6ff",
      "--code-bg": "#0c0c0e",
      "--code-block-bg": "#181819",
      "--inline-code-bg": "rgba(63, 63, 70, 0.4)",
      "--selection-bg": "oklch(0.7 0.1 285 / 20%)",
      "--table-row-alt": "#181819",
    },
    light: {
      "--background": "#fafafa",
      "--foreground": "#18181b",
      "--card": "#f4f4f5",
      "--card-foreground": "#18181b",
      "--primary": "#18181b",
      "--primary-foreground": "#fafafa",
      "--secondary": "#e4e4e7",
      "--secondary-foreground": "#18181b",
      "--muted": "#e4e4e7",
      "--muted-foreground": "#71717a",
      "--accent": "#d4d4d8",
      "--accent-foreground": "#18181b",
      "--border": "#d4d4d8",
      "--input": "#d4d4d8",
      "--ring": "#71717a",
      "--destructive": "#dc2626",
      "--success": "#16a34a",
      "--warning": "#ca8a04",
      "--scrollbar-thumb": "#a1a1aa",
      "--scrollbar-thumb-hover": "#71717a",
      "--link": "#2563eb",
      "--info": "#2563eb",
      "--code-bg": "#fafafa",
      "--code-block-bg": "#f4f4f5",
      "--inline-code-bg": "rgba(0, 0, 0, 0.06)",
      "--selection-bg": "rgba(59, 130, 246, 0.18)",
      "--table-row-alt": "#f4f4f5",
    },
  },
  {
    id: "ember",
    name: "Ember",
    description: "Warm amber Better Hub theme for review-heavy work sessions.",
    dark: {
      "--background": "#1a1008",
      "--foreground": "#faf0e4",
      "--card": "#231a0e",
      "--card-foreground": "#faf0e4",
      "--primary": "#f5e6d0",
      "--primary-foreground": "#1a1008",
      "--secondary": "#2a2014",
      "--secondary-foreground": "#faf0e4",
      "--muted": "#2a2014",
      "--muted-foreground": "#b8a48c",
      "--accent": "#2a2014",
      "--accent-foreground": "#faf0e4",
      "--border": "#3d2e1a",
      "--input": "#3d2e1a",
      "--ring": "#b45309",
      "--destructive": "#ef4444",
      "--success": "#22c55e",
      "--warning": "#f59e0b",
      "--scrollbar-thumb": "#3d2e1a",
      "--scrollbar-thumb-hover": "#4d3a22",
      "--link": "#f59e0b",
      "--info": "#60a5fa",
      "--code-bg": "#150d06",
      "--code-block-bg": "#231a0e",
      "--inline-code-bg": "rgba(180, 83, 9, 0.2)",
      "--selection-bg": "rgba(245, 158, 11, 0.20)",
      "--table-row-alt": "#231a0e",
    },
    light: {
      "--background": "#faf5ef",
      "--foreground": "#292524",
      "--card": "#f5ede4",
      "--card-foreground": "#292524",
      "--primary": "#292524",
      "--primary-foreground": "#faf5ef",
      "--secondary": "#f0e6d9",
      "--secondary-foreground": "#292524",
      "--muted": "#f0e6d9",
      "--muted-foreground": "#78716c",
      "--accent": "#ede0d1",
      "--accent-foreground": "#292524",
      "--border": "#ddd0c0",
      "--input": "#ddd0c0",
      "--ring": "#c2410c",
      "--destructive": "#dc2626",
      "--success": "#16a34a",
      "--warning": "#d97706",
      "--scrollbar-thumb": "#d4c4b0",
      "--scrollbar-thumb-hover": "#c4b49e",
      "--link": "#c2410c",
      "--info": "#2563eb",
      "--code-bg": "#faf5ef",
      "--code-block-bg": "#f0e6d9",
      "--inline-code-bg": "rgba(194, 65, 12, 0.08)",
      "--selection-bg": "rgba(194, 65, 12, 0.15)",
      "--table-row-alt": "#f5ede4",
    },
  },
  {
    id: "arctic",
    name: "Arctic",
    description: "Cool ice-blue Better Hub theme for high-contrast dashboards.",
    dark: {
      "--background": "#0c1425",
      "--foreground": "#e2e8f0",
      "--card": "#132038",
      "--card-foreground": "#e2e8f0",
      "--primary": "#cbd5e1",
      "--primary-foreground": "#0c1425",
      "--secondary": "#1a2d4e",
      "--secondary-foreground": "#e2e8f0",
      "--muted": "#1a2d4e",
      "--muted-foreground": "#94a3b8",
      "--accent": "#1a2d4e",
      "--accent-foreground": "#e2e8f0",
      "--border": "#1e3a5f",
      "--input": "#1e3a5f",
      "--ring": "#0284c7",
      "--destructive": "#f43f5e",
      "--success": "#2dd4bf",
      "--warning": "#fbbf24",
      "--scrollbar-thumb": "#1e3a5f",
      "--scrollbar-thumb-hover": "#2a4a70",
      "--link": "#38bdf8",
      "--info": "#38bdf8",
      "--code-bg": "#091020",
      "--code-block-bg": "#132038",
      "--inline-code-bg": "rgba(56, 189, 248, 0.12)",
      "--selection-bg": "rgba(56, 189, 248, 0.20)",
      "--table-row-alt": "#132038",
    },
    light: {
      "--background": "#f0f9ff",
      "--foreground": "#0c4a6e",
      "--card": "#e0f2fe",
      "--card-foreground": "#0c4a6e",
      "--primary": "#0c4a6e",
      "--primary-foreground": "#f0f9ff",
      "--secondary": "#bae6fd",
      "--secondary-foreground": "#0c4a6e",
      "--muted": "#bae6fd",
      "--muted-foreground": "#0369a1",
      "--accent": "#7dd3fc",
      "--accent-foreground": "#0c4a6e",
      "--border": "#7dd3fc",
      "--input": "#7dd3fc",
      "--ring": "#0284c7",
      "--destructive": "#e11d48",
      "--success": "#0d9488",
      "--warning": "#d97706",
      "--scrollbar-thumb": "#7dd3fc",
      "--scrollbar-thumb-hover": "#38bdf8",
      "--link": "#0284c7",
      "--info": "#0284c7",
      "--code-bg": "#f0f9ff",
      "--code-block-bg": "#e0f2fe",
      "--inline-code-bg": "rgba(2, 132, 199, 0.10)",
      "--selection-bg": "rgba(2, 132, 199, 0.18)",
      "--table-row-alt": "#e0f2fe",
    },
  },
];

export function useColorTheme(): ThemeControls {
  const [themeId, setThemeIdState] = useState<ThemeId>(() => {
    if (typeof window === "undefined") return DEFAULT_THEME_ID;
    const stored = window.localStorage.getItem(THEME_STORAGE_KEY);
    return isThemeId(stored) ? stored : DEFAULT_THEME_ID;
  });
  const [mode, setMode] = useState<ColorMode>(() => {
    if (typeof window === "undefined") return "dark";
    const stored = window.localStorage.getItem(MODE_STORAGE_KEY);
    if (stored === "light" || stored === "dark") return stored;
    return window.matchMedia?.("(prefers-color-scheme: light)")?.matches ? "light" : "dark";
  });
  const [radius, setRadiusState] = useState<RadiusPreset>(() => {
    if (typeof window === "undefined") return DEFAULT_RADIUS;
    const stored = window.localStorage.getItem(RADIUS_STORAGE_KEY);
    return isRadiusPreset(stored) ? stored : DEFAULT_RADIUS;
  });

  useEffect(() => {
    const theme = THEMES.find((candidate) => candidate.id === themeId) ?? THEMES[0];
    applyTheme(theme[mode], RADIUS_PRESETS[radius]);
    document.documentElement.dataset.theme = mode;
    document.documentElement.dataset.colorTheme = theme.id;
    document.documentElement.dataset.radius = radius;
    document.documentElement.classList.toggle("dark", mode === "dark");
    document.documentElement.style.colorScheme = mode;
    window.localStorage.setItem(THEME_STORAGE_KEY, theme.id);
    window.localStorage.setItem(MODE_STORAGE_KEY, mode);
    window.localStorage.setItem(RADIUS_STORAGE_KEY, radius);
  }, [mode, radius, themeId]);

  return {
    mode,
    radius,
    setMode,
    setRadius: setRadiusState,
    setThemeId: setThemeIdState,
    themeId,
    themes: THEMES,
    toggleMode: () => setMode((current) => (current === "dark" ? "light" : "dark")),
  };
}

function applyTheme(colors: ThemeColors, radius: Record<string, string>) {
  const root = document.documentElement;
  for (const [property, value] of Object.entries(colors)) root.style.setProperty(property, value);
  for (const [property, value] of Object.entries(radius)) root.style.setProperty(property, value);

  const surfaceMuted = colors["--muted"] ?? colors["--secondary"];
  const mutedForeground = colors["--muted-foreground"];
  root.style.setProperty("--bg", colors["--background"]);
  root.style.setProperty("--surface", colors["--card"]);
  root.style.setProperty("--surface-muted", colors["--secondary"]);
  root.style.setProperty("--surface-soft", `color-mix(in srgb, ${colors["--secondary"]} 76%, ${colors["--foreground"]})`);
  root.style.setProperty("--fg", colors["--foreground"]);
  root.style.setProperty("--muted-surface", surfaceMuted);
  root.style.setProperty("--muted", mutedForeground);
  root.style.setProperty("--faint", `color-mix(in srgb, ${mutedForeground} 62%, transparent)`);
  root.style.setProperty("--border-soft", `color-mix(in srgb, ${colors["--border"]} 72%, transparent)`);
  root.style.setProperty("--accent", colors["--primary"]);
  root.style.setProperty("--accent-soft", `color-mix(in srgb, ${colors["--primary"]} 12%, ${colors["--secondary"]})`);
  root.style.setProperty("--green", colors["--success"]);
  root.style.setProperty("--green-soft", `color-mix(in srgb, ${colors["--success"]} 12%, ${colors["--secondary"]})`);
  root.style.setProperty("--blue", colors["--info"]);
  root.style.setProperty("--blue-soft", `color-mix(in srgb, ${colors["--info"]} 12%, ${colors["--secondary"]})`);
  root.style.setProperty("--red", colors["--destructive"]);
  root.style.setProperty("--red-soft", `color-mix(in srgb, ${colors["--destructive"]} 13%, ${colors["--secondary"]})`);
}

function isThemeId(value: string | null): value is ThemeId {
  return value === "better-auth" || value === "zinc" || value === "ember" || value === "arctic";
}

function isRadiusPreset(value: string | null): value is RadiusPreset {
  return value === "default" || value === "small" || value === "medium" || value === "large";
}
