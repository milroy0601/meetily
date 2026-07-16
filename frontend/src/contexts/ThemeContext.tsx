'use client';

import React, { createContext, useContext, useState, useEffect, useCallback } from 'react';

export type ThemeId = 'light' | 'dark' | 'cyberpunk' | 'retro';

interface ThemeDefinition {
  id: ThemeId;
  label: string;
  icon: string;
  description: string;
}

export const THEMES: ThemeDefinition[] = [
  { id: 'light', label: 'Light', icon: '☀️', description: 'Clean and bright' },
  { id: 'dark', label: 'Dark', icon: '🌙', description: 'Easy on the eyes' },
  { id: 'cyberpunk', label: 'Cyberpunk', icon: '⚡', description: 'Neon dystopia' },
  { id: 'retro', label: 'Typewriter', icon: '🖋️', description: 'Vintage paper & ink' },
];

const THEME_CLASS_PREFIX = 'theme-';
const STORAGE_KEY = 'meetily-theme';

function applyThemeClass(theme: ThemeId): void {
  const root = document.documentElement;
  // Remove all theme classes
  THEMES.forEach((t) => root.classList.remove(`${THEME_CLASS_PREFIX}${t.id}`));
  // Remove legacy .dark class (re-added below if needed)
  root.classList.remove('dark');
  // Add current theme class
  root.classList.add(`${THEME_CLASS_PREFIX}${theme}`);
  // Also add .dark for dark-based themes so Tailwind dark: prefixes work
  if (theme === 'dark' || theme === 'cyberpunk') {
    root.classList.add('dark');
  }
}

interface ThemeContextValue {
  theme: ThemeId;
  setTheme: (theme: ThemeId) => void;
  themes: ThemeDefinition[];
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const [theme, setThemeState] = useState<ThemeId>('light');
  const [mounted, setMounted] = useState(false);

  // On mount, read from localStorage
  useEffect(() => {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored && THEMES.some((t) => t.id === stored)) {
        setThemeState(stored as ThemeId);
        applyThemeClass(stored as ThemeId);
      } else {
        // Default: respect system preference for light/dark, then fall back to light
        const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        const defaultTheme: ThemeId = prefersDark ? 'dark' : 'light';
        setThemeState(defaultTheme);
        applyThemeClass(defaultTheme);
      }
    } catch {
      applyThemeClass('light');
    }
    setMounted(true);
  }, []);

  const setTheme = useCallback((newTheme: ThemeId) => {
    setThemeState(newTheme);
    try {
      localStorage.setItem(STORAGE_KEY, newTheme);
    } catch { /* noop */ }
    applyThemeClass(newTheme);
  }, []);

  // Avoid SSR mismatch by rendering nothing until mounted
  if (!mounted) {
    return <>{children}</>;
  }

  return (
    <ThemeContext.Provider value={{ theme, setTheme, themes: THEMES }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return ctx;
}
