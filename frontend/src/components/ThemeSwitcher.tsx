'use client';

import React from 'react';
import { useTheme, ThemeId, THEMES } from '@/contexts/ThemeContext';
import { Paintbrush } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';

export function ThemeSwitcher() {
  const { theme, setTheme } = useTheme();

  const currentTheme = THEMES.find((t) => t.id === theme) ?? THEMES[0];

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          title={`Theme: ${currentTheme.label}`}
          aria-label="Switch theme"
        >
          <Paintbrush className="h-4 w-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-[180px]">
        {THEMES.map((t) => (
          <DropdownMenuItem
            key={t.id}
            onClick={() => setTheme(t.id)}
            className="flex items-center gap-3 cursor-pointer"
          >
            <span className="text-lg">{t.icon}</span>
            <div className="flex flex-col">
              <span className={`text-sm font-medium ${t.id === theme ? 'text-primary' : ''}`}>
                {t.label}
                {t.id === theme && ' ✓'}
              </span>
              <span className="text-xs text-muted-foreground">{t.description}</span>
            </div>
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Compact row of theme dots for settings panels */
export function ThemeDots({ className }: { className?: string }) {
  const { theme, setTheme } = useTheme();

  return (
    <div className={`flex items-center gap-1.5 ${className ?? ''}`}>
      {THEMES.map((t) => {
        const isActive = t.id === theme;
        const colorMap: Record<ThemeId, string> = {
          light: 'bg-card border-border',
          dark: 'bg-gray-800 border-gray-600',
          cyberpunk: 'bg-fuchsia-600 border-cyan-400',
          retro: 'bg-amber-100 border-amber-700',
        };
        return (
          <button
            key={t.id}
            onClick={() => setTheme(t.id)}
            title={t.label}
            className={`w-6 h-6 rounded-full border-2 transition-all ${colorMap[t.id]} ${
              isActive ? 'scale-110 ring-2 ring-offset-1 ring-primary' : 'opacity-60 hover:opacity-100'
            }`}
          />
        );
      })}
    </div>
  );
}
