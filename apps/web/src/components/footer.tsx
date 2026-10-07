import Image from "next/image";
import Link from "next/link";
import type { Locale } from "@/i18n/config";
import { fmt } from "@/i18n/config";
import type { Dictionary } from "@/i18n";
import { Container } from "./ui";

interface FooterProps {
  locale: Locale;
  dict: Pick<Dictionary, "footer" | "nav">;
  githubUrl: string | undefined;
}

export function Footer({ locale, dict, githubUrl }: FooterProps) {
  const { footer, nav } = dict;
  const home = `/${locale}`;
  const link = "focus-ring text-fg-2 hover:text-fg rounded transition-colors";
  return (
    <footer className="mt-24 border-t border-[var(--separator)] py-12">
      <Container className="flex flex-col gap-10 sm:flex-row sm:justify-between">
        <div className="max-w-xs">
          <Link
            href={home}
            className="inline-flex items-center gap-2.5 rounded-full focus-ring"
            aria-label={nav.home}
          >
            <Image src="/logo.png" alt="" width={28} height={28} className="size-7 rounded-[8px]" />
            <span className="text-[17px] font-semibold tracking-tight">VynPad</span>
          </Link>
          <p className="mt-3 text-sm text-fg-2">{footer.tagline}</p>
        </div>
        <div className="grid grid-cols-2 gap-x-16 gap-y-6 text-sm">
          <div>
            <p className="mb-3 text-xs font-semibold tracking-wide text-fg-3 uppercase">
              {footer.product}
            </p>
            <ul className="space-y-2">
              <li>
                <a className={link} href={`${home}#download`}>
                  {nav.download}
                </a>
              </li>
              <li>
                <Link className={link} href={`${home}/changelog`}>
                  {footer.changelog}
                </Link>
              </li>
              {githubUrl && (
                <li>
                  <a className={link} href={githubUrl} rel="noopener noreferrer">
                    {footer.source}
                  </a>
                </li>
              )}
            </ul>
          </div>
          <div>
            <p className="mb-3 text-xs font-semibold tracking-wide text-fg-3 uppercase">
              {footer.legal}
            </p>
            <ul className="space-y-2">
              <li>
                <Link className={link} href={`${home}/privacy`}>
                  {footer.privacy}
                </Link>
              </li>
            </ul>
          </div>
        </div>
      </Container>
      <Container className="mt-10">
        <p className="text-xs text-fg-3">
          {fmt(footer.rights, { year: new Date().getFullYear() })}
        </p>
      </Container>
    </footer>
  );
}
