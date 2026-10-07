import { NextResponse, type NextRequest } from "next/server";
import { LOCALE_COOKIE, LOCALES, negotiateLocale } from "@/i18n/config";

/**
 * Toda página mora sob `/pt-BR` ou `/en`. Quem chega sem idioma na URL é mandado para o seu
 * (cookie, depois `Accept-Language`). O resto (API, arquivos estáticos, ícones) não passa por aqui.
 */
export function proxy(request: NextRequest): NextResponse {
  const { pathname } = request.nextUrl;
  const hasLocale = LOCALES.some((l) => pathname === `/${l}` || pathname.startsWith(`/${l}/`));
  if (hasLocale) return NextResponse.next();

  const locale = negotiateLocale(
    request.headers.get("accept-language"),
    request.cookies.get(LOCALE_COOKIE)?.value,
  );
  const url = request.nextUrl.clone();
  url.pathname = `/${locale}${pathname === "/" ? "" : pathname}`;
  const response = NextResponse.redirect(url, 307);
  // O idioma muda a resposta: caches intermediários não podem reaproveitá-la entre idiomas.
  response.headers.set("Vary", "Accept-Language, Cookie");
  return response;
}

export const config = {
  // Fora: API, internos do Next e qualquer arquivo com extensão (ícones, robots, sitemap...).
  matcher: ["/((?!api|_next|.*\\..*).*)"],
};
