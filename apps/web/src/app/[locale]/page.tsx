import { notFound } from "next/navigation";
import { Download } from "@/components/download";
import { FinalCta } from "@/components/final-cta";
import { Hero } from "@/components/hero";
import { Faq, Features, HowItWorks, ProofStrip, Tech } from "@/components/sections";
import { getDictionary } from "@/i18n";
import { isLocale } from "@/i18n/config";
import { readConfig } from "@/lib/config";
import { getLatestRelease } from "@/lib/github";
import { toDownloadView } from "@/lib/release-view";

// O release é lido do GitHub e guardado por alguns minutos; a página é refeita nesse ritmo.
export const revalidate = 300;

export default async function HomePage({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  if (!isLocale(locale)) notFound();
  const dict = getDictionary(locale);
  const config = readConfig();
  const view = toDownloadView(await getLatestRelease(config), config);

  // Dados estruturados para buscadores. Sem preço: ainda não há oferta pública definida.
  const jsonLd = {
    "@context": "https://schema.org",
    "@type": "SoftwareApplication",
    name: "VynPad",
    description: dict.meta.description,
    applicationCategory: "UtilitiesApplication",
    operatingSystem: "Windows 10, Windows 11, Android 8.0+",
    inLanguage: locale,
    url: `${config.siteUrl}/${locale}`,
    ...(view.version ? { softwareVersion: view.version } : {}),
  };

  return (
    <>
      <script
        type="application/ld+json"
        // O conteúdo é montado aqui, sem entrada do visitante; `<` é escapado por garantia.
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd).replace(/</g, "\u003c") }}
      />
      <Hero locale={locale} dict={dict} view={view} />
      <ProofStrip dict={dict} />
      <HowItWorks dict={dict} />
      <Features dict={dict} />
      <Tech dict={dict} />
      <Download locale={locale} dict={dict} view={view} />
      <Faq dict={dict} />
      <FinalCta dict={dict} />
    </>
  );
}
