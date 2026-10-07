"use client";

import { ArrowDown, Download, Smartphone } from "lucide-react";
import type { Dictionary } from "@/i18n";
import { fmt } from "@/i18n/config";
import type { DownloadView } from "@/lib/release-view";
import { OS_NAMES } from "@/lib/os";
import { useOs } from "./use-os";
import { ButtonLink } from "./ui";

interface HeroActionsProps {
  view: DownloadView;
  dict: {
    hero: Pick<Dictionary["hero"], "ctaDownload" | "ctaDownloadGeneric" | "ctaHow">;
    download: { android: Pick<Dictionary["download"]["android"], "play"> };
  };
}

/**
 * O botão principal muda conforme o sistema de quem visita: no Windows baixa o instalador direto;
 * no Android leva à Google Play (quando existe); nos demais, à seção de download.
 */
export function HeroActions({ view, dict }: HeroActionsProps) {
  const os = useOs();
  const { hero, download } = dict;
  const hasWindows = view.files.some((f) => f.platform === "windows");

  let primary = {
    href: "#download",
    label: hero.ctaDownloadGeneric,
    icon: <ArrowDown aria-hidden className="size-[18px]" strokeWidth={2} />,
  };
  if (os === "windows" && hasWindows) {
    primary = {
      href: "/api/download/windows",
      label: fmt(hero.ctaDownload, { os: OS_NAMES.windows }),
      icon: <Download aria-hidden className="size-[18px]" strokeWidth={2} />,
    };
  } else if (os === "android" && view.playAvailable) {
    primary = {
      href: "/api/download/android",
      label: download.android.play,
      icon: <Smartphone aria-hidden className="size-[18px]" strokeWidth={2} />,
    };
  }

  return (
    <div className="mt-9 flex flex-wrap items-center gap-3">
      <ButtonLink href={primary.href} size="lg">
        {primary.icon}
        {primary.label}
      </ButtonLink>
      <ButtonLink href="#how" variant="secondary" size="lg">
        {hero.ctaHow}
      </ButtonLink>
    </div>
  );
}
