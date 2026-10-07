"use client";

import { Monitor, Moon, Sun } from "lucide-react";
import { motion } from "motion/react";
import { useEffect, useState } from "react";
import { cn } from "@/lib/cn";
import { THEME_KEY } from "./theme-script";

type Mode = "system" | "dark" | "light";

interface ThemeToggleProps {
  labels: { label: string; system: string; dark: string; light: string };
}

function resolve(mode: Mode): "dark" | "light" {
  if (mode !== "system") return mode;
  return matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

function apply(mode: Mode) {
  document.documentElement.dataset.theme = resolve(mode);
}

const OPTIONS = [
  { mode: "system", Icon: Monitor },
  { mode: "dark", Icon: Moon },
  { mode: "light", Icon: Sun },
] as const;

/** Segmentado de três posições (automático, escuro, claro) com a pílula deslizando por mola. */
export function ThemeToggle({ labels }: ThemeToggleProps) {
  const [mode, setMode] = useState<Mode>("system");

  useEffect(() => {
    try {
      const saved = localStorage.getItem(THEME_KEY);
      // Ler o que foi guardado só é possível no navegador, depois da hidratação.
      // eslint-disable-next-line react-hooks/set-state-in-effect
      if (saved === "dark" || saved === "light") setMode(saved);
    } catch {
      /* sem armazenamento: fica no automático */
    }
  }, []);

  // No automático, acompanha o sistema se ele mudar com a página aberta.
  useEffect(() => {
    if (mode !== "system") return;
    const query = matchMedia("(prefers-color-scheme: light)");
    const onChange = () => {
      apply("system");
    };
    query.addEventListener("change", onChange);
    return () => {
      query.removeEventListener("change", onChange);
    };
  }, [mode]);

  const choose = (next: Mode) => {
    setMode(next);
    apply(next);
    try {
      if (next === "system") localStorage.removeItem(THEME_KEY);
      else localStorage.setItem(THEME_KEY, next);
    } catch {
      /* sem armazenamento: vale só nesta visita */
    }
  };

  return (
    <div
      role="radiogroup"
      aria-label={labels.label}
      className="relative inline-flex h-9 items-center rounded-full bg-control p-0.5"
    >
      {OPTIONS.map(({ mode: value, Icon }) => {
        const active = mode === value;
        const name = labels[value];
        return (
          <button
            key={value}
            type="button"
            role="radio"
            aria-checked={active}
            aria-label={name}
            title={name}
            onClick={() => {
              choose(value);
            }}
            className={cn(
              "relative grid size-8 place-items-center rounded-full focus-ring transition-colors",
              active ? "text-fg" : "text-fg-3 hover:text-fg-2",
            )}
          >
            {active && (
              <motion.span
                layoutId="theme-pill"
                transition={{ type: "spring", stiffness: 380, damping: 32 }}
                className="absolute inset-0 rounded-full bg-page shadow-sm"
              />
            )}
            <Icon aria-hidden className="relative size-4" strokeWidth={1.75} />
          </button>
        );
      })}
    </div>
  );
}
