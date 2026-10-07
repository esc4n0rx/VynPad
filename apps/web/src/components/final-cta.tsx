import type { Dictionary } from "@/i18n";
import { Reveal } from "./reveal";
import { ButtonLink, Container } from "./ui";

export function FinalCta({ dict }: { dict: Pick<Dictionary, "cta"> }) {
  const { cta } = dict;
  return (
    <section aria-labelledby="cta-title" className="py-12">
      <Container>
        <Reveal>
          <div className="relative overflow-hidden rounded-[2rem] px-6 py-16 text-center glass hairline sm:px-12 sm:py-20">
            <div
              aria-hidden
              className="pointer-events-none absolute inset-0 bg-[radial-gradient(60%_90%_at_50%_120%,color-mix(in_srgb,var(--accent)_28%,transparent),transparent)]"
            />
            <h2
              id="cta-title"
              className="relative mx-auto max-w-2xl text-h2 font-semibold text-balance"
            >
              {cta.title}
            </h2>
            <p className="relative mx-auto mt-4 max-w-lg text-lead text-fg-2">{cta.body}</p>
            <div className="relative mt-9">
              <ButtonLink href="#download" size="lg">
                {cta.button}
              </ButtonLink>
            </div>
          </div>
        </Reveal>
      </Container>
    </section>
  );
}
