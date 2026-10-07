"use client";

/* eslint-disable @next/next/no-html-link-for-pages -- os links /api/download/* são rotas de arquivo, não páginas: um <Link> as pré-carregaria (e dispararia o download no servidor). */

import { Check, Copy, Download as DownloadIcon, Monitor, Smartphone, Terminal } from "lucide-react";
import Link from "next/link";
import { useState, useSyncExternalStore, type ReactNode } from "react";
import type { Dictionary } from "@/i18n";
import { intlLocale } from "@/i18n";
import type { Locale } from "@/i18n/config";
import { cn } from "@/lib/cn";
import type { Os } from "@/lib/os";
import type { DownloadFile, DownloadView } from "@/lib/release-view";
import { useOs } from "./use-os";
import { ButtonLink } from "./ui";

interface DownloadCardsProps {
  locale: Locale;
  dict: Dictionary["download"];
  view: DownloadView;
}

export function formatSize(bytes: number, locale: string): string {
  const mb = bytes / (1024 * 1024);
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: mb >= 100 ? 0 : 1 }).format(mb)} MB`;
}

export function formatDate(iso: string | undefined, locale: string): string | undefined {
  if (!iso) return undefined;
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return undefined;
  return new Intl.DateTimeFormat(locale, { day: "2-digit", month: "long", year: "numeric" }).format(
    date,
  );
}

const subscribe = () => () => undefined;

/** `?download=unavailable|error`: a rota de download devolve para cá quando algo falha. */
function useDownloadFlag(): string | null {
  return useSyncExternalStore(
    subscribe,
    () => new URLSearchParams(window.location.search).get("download"),
    () => null,
  );
}

function Card({
  recommended,
  recommendedLabel,
  icon,
  name,
  requirements,
  primary = false,
  children,
}: {
  recommended: boolean;
  recommendedLabel: string;
  icon: ReactNode;
  name: string;
  requirements: string;
  primary?: boolean;
  children: ReactNode;
}) {
  return (
    <article
      className={cn(
        "relative flex flex-col rounded-3xl p-6 glass hairline sm:p-7",
        recommended && "ring-2 ring-accent",
        primary && "lg:row-span-1",
      )}
    >
      {recommended && (
        <span className="absolute -top-3 left-6 rounded-full bg-accent px-3 py-1 text-xs font-semibold text-on-accent">
          {recommendedLabel}
        </span>
      )}
      <div className="flex items-center gap-3">
        <span className="grid size-11 place-items-center rounded-2xl bg-control">{icon}</span>
        <div>
          <h3 className="text-lg leading-tight font-semibold tracking-tight">{name}</h3>
          <p className="text-sm text-fg-3">{requirements}</p>
        </div>
      </div>
      {children}
    </article>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-xs text-fg-3">{label}</dt>
      <dd className="mt-0.5 text-sm font-semibold tabular-nums">{value}</dd>
    </div>
  );
}

function Checksum({ sha, labels }: { sha: string; labels: Dictionary["download"]["meta"] }) {
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(sha);
      setCopied(true);
      setTimeout(() => {
        setCopied(false);
      }, 2000);
    } catch {
      /* sem permissão para a área de transferência: a hash continua selecionável */
    }
  };
  return (
    <div className="mt-4 rounded-2xl bg-control p-3">
      <div className="flex items-center justify-between gap-2">
        <span className="text-xs font-semibold text-fg-3">{labels.sha}</span>
        <button
          type="button"
          onClick={() => {
            void copy();
          }}
          className="inline-flex items-center gap-1.5 rounded-full px-2 py-1 text-xs font-medium text-fg-2 focus-ring hover:text-fg"
        >
          {copied ? (
            <Check className="size-3.5" aria-hidden />
          ) : (
            <Copy className="size-3.5" aria-hidden />
          )}
          <span aria-live="polite">{copied ? labels.copied : labels.copy}</span>
        </button>
      </div>
      <code className="mt-1 block font-mono text-[11px] leading-relaxed break-all text-fg-2 select-all">
        {sha}
      </code>
    </div>
  );
}

export function DownloadCards({ locale, dict, view }: DownloadCardsProps) {
  const os: Os = useOs();
  const flag = useDownloadFlag();
  const intl = intlLocale(locale);
  const file = (platform: DownloadFile["platform"]) =>
    view.files.find((f) => f.platform === platform);
  const win = file("windows");
  const msi = file("windows-msi");
  const published = formatDate(view.publishedAt, intl);

  const banner = flag === "error" ? dict.error : flag === "unavailable" ? dict.unavailable : null;

  return (
    <div>
      {banner && (
        <p role="status" className="mb-6 rounded-2xl bg-control px-4 py-3 text-sm text-fg-2">
          {banner}
        </p>
      )}
      <div className="grid gap-5 pt-3 lg:grid-cols-[1.25fr_1fr_1fr]">
        {/* Windows */}
        <Card
          recommended={os === "windows"}
          recommendedLabel={dict.recommended}
          icon={<Monitor className="size-5" strokeWidth={1.75} />}
          name={dict.windows.name}
          requirements={dict.windows.requirements}
          primary
        >
          {view.status === "ok" && win ? (
            <>
              <dl className="mt-6 grid grid-cols-3 gap-4">
                <Fact label={dict.meta.version} value={view.version ?? ""} />
                {published && <Fact label={dict.meta.published} value={published} />}
                <Fact label={dict.meta.size} value={formatSize(win.size, intl)} />
              </dl>
              {view.prerelease && (
                <p className="mt-3 text-xs font-semibold text-warning">{dict.prerelease}</p>
              )}
              <div className="mt-6 flex flex-wrap items-center gap-3">
                <ButtonLink href="/api/download/windows" size="lg" className="grow sm:grow-0">
                  <DownloadIcon aria-hidden className="size-[18px]" strokeWidth={2} />
                  {dict.windows.button}
                </ButtonLink>
                {msi && (
                  <a
                    href="/api/download/windows-msi"
                    className="rounded text-sm font-medium text-fg-2 underline-offset-4 focus-ring hover:text-fg hover:underline"
                  >
                    {dict.windows.altMsi}
                  </a>
                )}
              </div>
              {win.sha256 && <Checksum sha={win.sha256} labels={dict.meta} />}
              <p className="mt-4 text-xs text-pretty text-fg-3">{dict.verify}</p>
            </>
          ) : (
            <p className="mt-6 text-[15px] text-pretty text-fg-2">
              {view.status === "error" ? dict.error : dict.empty}
            </p>
          )}
          {view.status === "ok" && (
            <Link
              href={`/${locale}/changelog`}
              className="mt-auto rounded pt-5 text-sm font-medium text-accent focus-ring hover:underline"
            >
              {dict.meta.notes}
            </Link>
          )}
        </Card>

        {/* Android */}
        <Card
          recommended={os === "android"}
          recommendedLabel={dict.recommended}
          icon={<Smartphone className="size-5" strokeWidth={1.75} />}
          name={dict.android.name}
          requirements={dict.android.requirements}
        >
          {view.playAvailable ? (
            <div className="mt-6">
              <ButtonLink href="/api/download/android" size="lg" className="w-full">
                {dict.android.play}
              </ButtonLink>
            </div>
          ) : (
            <>
              <span
                aria-disabled
                className="mt-6 inline-flex h-12 items-center justify-center rounded-full bg-control px-6 text-base font-semibold text-fg-2"
              >
                {dict.android.soon}
              </span>
              <p className="mt-4 text-sm text-pretty text-fg-3">{dict.android.soonNote}</p>
            </>
          )}
        </Card>

        {/* Linux */}
        <Card
          recommended={os === "linux"}
          recommendedLabel={dict.recommended}
          icon={<Terminal className="size-5" strokeWidth={1.75} />}
          name={dict.linux.name}
          requirements={dict.linux.requirements}
        >
          <span
            aria-disabled
            className="mt-6 inline-flex h-12 items-center justify-center rounded-full bg-control px-6 text-base font-semibold text-fg-2"
          >
            {dict.linux.soon}
          </span>
          <p className="mt-4 text-sm text-pretty text-fg-3">{dict.linux.soonNote}</p>
        </Card>
      </div>
    </div>
  );
}
