import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { ImageResponse } from "next/og";
import { getDictionary } from "@/i18n";
import { isLocale, LOCALES } from "@/i18n/config";

// Imagem de compartilhamento (Open Graph / redes). Roda fora do CSS do site, por isso os valores
// aqui são os mesmos tokens do design system, escritos por extenso.
export const alt = "VynPad";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

export function generateStaticParams() {
  return LOCALES.map((locale) => ({ locale }));
}

export default async function OpengraphImage({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const dict = getDictionary(isLocale(locale) ? locale : "pt-BR");
  const logo = `data:image/png;base64,${(await readFile(join(process.cwd(), "public", "logo.png"))).toString("base64")}`;

  return new ImageResponse(
    <div
      style={{
        width: "100%",
        height: "100%",
        display: "flex",
        flexDirection: "column",
        justifyContent: "space-between",
        padding: 72,
        color: "rgba(255,255,255,0.94)",
        background:
          "radial-gradient(900px 600px at 0% 0%, rgba(10,132,255,0.30), transparent 70%), radial-gradient(800px 600px at 100% 100%, rgba(94,92,230,0.26), transparent 70%), #1c1c1e",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 20 }}>
        {/* eslint-disable-next-line @next/next/no-img-element -- ImageResponse não usa next/image */}
        <img src={logo} width={64} height={64} alt="" style={{ borderRadius: 16 }} />
        <div style={{ fontSize: 40, fontWeight: 700, letterSpacing: -1 }}>VynPad</div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
        <div
          style={{
            fontSize: 84,
            fontWeight: 700,
            lineHeight: 1.04,
            letterSpacing: -3,
            maxWidth: 980,
          }}
        >
          {dict.hero.title}
        </div>
        <div style={{ fontSize: 30, color: "rgba(235,235,245,0.62)", maxWidth: 900 }}>
          {dict.hero.note}
        </div>
      </div>
    </div>,
    size,
  );
}
