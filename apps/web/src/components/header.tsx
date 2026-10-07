"use client";

import { Menu, X } from "lucide-react";
import Image from "next/image";
import Link from "next/link";
import { useEffect, useState } from "react";
import type { Locale } from "@/i18n/config";
import type { Dictionary } from "@/i18n";
import { cn } from "@/lib/cn";
import { LocaleSwitch } from "./locale-switch";
import { ThemeToggle } from "./theme-toggle";
import { buttonClass } from "./ui";

interface HeaderProps {
  locale: Locale;
  dict: Pick<Dictionary, "nav" | "theme" | "locale">;
}

const LINKS = ["features", "how", "tech", "faq"] as const;

/** Barra flutuante de vidro, como a janela do app. No celular, os links viram um painel. */
export function Header({ locale, dict }: HeaderProps) {
  const [open, setOpen] = useState(false);
  const home = `/${locale}`;
  const { nav } = dict;

  // Esc fecha o painel; abrir o painel não deixa a página rolar por baixo.
  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const close = () => {
    setOpen(false);
  };

  return (
    <header className="sticky top-0 z-50 px-3 pt-3 sm:px-5">
      <div className="mx-auto flex h-14 max-w-[72rem] items-center gap-3 rounded-full pr-2 pl-4 glass-bar hairline sm:pl-5">
        <Link
          href={home}
          aria-label={nav.home}
          className="flex items-center gap-2.5 rounded-full focus-ring"
        >
          <Image
            src="/logo.png"
            alt=""
            width={28}
            height={28}
            className="size-7 rounded-[8px]"
            priority
          />
          <span className="text-[17px] font-semibold tracking-tight">VynPad</span>
        </Link>

        <nav aria-label={nav.label} className="mx-auto hidden items-center gap-1 lg:flex">
          {LINKS.map((key) => (
            <a
              key={key}
              href={`${home}#${key}`}
              className="rounded-full px-3.5 py-1.5 text-sm font-medium text-fg-2 focus-ring transition-colors hover:text-fg"
            >
              {nav[key]}
            </a>
          ))}
        </nav>

        <div className="ml-auto flex items-center gap-2 lg:ml-0">
          <div className="hidden items-center gap-2 lg:flex">
            <LocaleSwitch current={locale} labels={dict.locale} />
            <ThemeToggle labels={dict.theme} />
          </div>
          <a
            href={`${home}#download`}
            className={buttonClass("primary", "md", "hidden sm:inline-flex")}
          >
            {nav.download}
          </a>
          <button
            type="button"
            aria-expanded={open}
            aria-controls="mobile-menu"
            aria-label={open ? nav.menuClose : nav.menuOpen}
            onClick={() => {
              setOpen((v) => !v);
            }}
            className="grid size-10 place-items-center rounded-full bg-control focus-ring lg:hidden"
          >
            {open ? (
              <X className="size-5" strokeWidth={1.75} />
            ) : (
              <Menu className="size-5" strokeWidth={1.75} />
            )}
          </button>
        </div>
      </div>

      <div
        id="mobile-menu"
        hidden={!open}
        className={cn(
          "mx-auto mt-2 max-w-[72rem] origin-top rounded-3xl p-3 glass-bar hairline lg:hidden",
          open && "animate-[menu-in_220ms_ease-out]",
        )}
      >
        <nav aria-label={nav.label} className="flex flex-col">
          {[...LINKS, "download" as const].map((key) => (
            <a
              key={key}
              href={`${home}#${key}`}
              onClick={close}
              className="rounded-2xl px-4 py-3 text-base font-medium focus-ring hover:bg-control"
            >
              {nav[key]}
            </a>
          ))}
        </nav>
        <div className="mt-2 flex items-center justify-between gap-3 border-t border-[var(--separator)] px-2 pt-3">
          <LocaleSwitch current={locale} labels={dict.locale} />
          <ThemeToggle labels={dict.theme} />
        </div>
      </div>
    </header>
  );
}
