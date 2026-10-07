import type { Dictionary } from "@/i18n";
import type { Locale } from "@/i18n/config";
import type { DownloadView } from "@/lib/release-view";
import { DownloadCards } from "./download-cards";
import { Reveal } from "./reveal";
import { Container, SectionHeading } from "./ui";

interface DownloadProps {
  locale: Locale;
  dict: Pick<Dictionary, "download">;
  view: DownloadView;
}

export function Download({ locale, dict, view }: DownloadProps) {
  const { download } = dict;
  return (
    <section id="download" aria-labelledby="download-title" className="py-24">
      <Container>
        <Reveal>
          <SectionHeading
            id="download-title"
            eyebrow={download.eyebrow}
            title={download.title}
            lead={download.lead}
          />
        </Reveal>
        <Reveal delay={0.06} className="mt-12">
          <DownloadCards locale={locale} dict={download} view={view} />
        </Reveal>
      </Container>
    </section>
  );
}
