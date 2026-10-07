import { ArrowRight } from "lucide-react";
import Link from "next/link";
import type { Dictionary } from "@/i18n";
import { fmt, type Locale } from "@/i18n/config";
import type { DownloadView } from "@/lib/release-view";
import { HeroActions } from "./hero-actions";
import { HeroScene } from "./hero-scene";
import { Container } from "./ui";

interface HeroProps {
  locale: Locale;
  dict: Pick<Dictionary, "hero" | "download">;
  view: DownloadView;
}

export function Hero({ locale, dict, view }: HeroProps) {
  const { hero } = dict;
  const label = view.version
    ? fmt(hero.eyebrowVersion, { version: view.version })
    : hero.eyebrowSoon;
  return (
    <section aria-labelledby="hero-title" className="pt-10 pb-8 sm:pt-16 lg:pt-20">
      <Container className="grid items-center gap-14 lg:grid-cols-[minmax(0,0.92fr)_minmax(0,1.08fr)] lg:gap-8">
        <div>
          <Link
            href={`/${locale}/changelog`}
            className="group mb-7 inline-flex items-center gap-2 rounded-full py-1.5 pr-3 pl-3.5 text-sm font-medium text-fg-2 glass hairline focus-ring transition-colors hover:text-fg"
          >
            <span
              className="size-1.5 rounded-full bg-success shadow-[0_0_8px_var(--success)]"
              aria-hidden
            />
            {label}
            <ArrowRight
              aria-hidden
              className="size-3.5 transition-transform group-hover:translate-x-0.5"
              strokeWidth={2}
            />
          </Link>
          <h1 id="hero-title" className="text-display font-semibold text-balance">
            {hero.title}
          </h1>
          <p className="mt-6 max-w-[34rem] text-lead text-pretty text-fg-2">{hero.lead}</p>
          <HeroActions
            view={view}
            dict={{
              hero: {
                ctaDownload: hero.ctaDownload,
                ctaDownloadGeneric: hero.ctaDownloadGeneric,
                ctaHow: hero.ctaHow,
              },
              download: { android: { play: dict.download.android.play } },
            }}
          />
          <p className="mt-5 text-sm text-fg-3">{hero.note}</p>
        </div>
        <div className="mx-auto w-full max-w-[40rem] lg:max-w-none">
          <HeroScene dict={{ hero }} />
        </div>
      </Container>
    </section>
  );
}
