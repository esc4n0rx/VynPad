import type { ReactNode } from "react";
import type { Dictionary } from "@/i18n";
import { fmt } from "@/i18n/config";
import { cn } from "@/lib/cn";
import { GamepadArt } from "./gamepad-art";
import {
  GyroArt,
  LatencyArt,
  LayoutsArt,
  PrivacyArt,
  QrArt,
  RumbleArt,
  SlotsArt,
  WizardArt,
} from "./feature-art";
import { Reveal } from "./reveal";
import { Container, SectionHeading } from "./ui";

// ---------------------------------------------------------------------------- provas

export function ProofStrip({ dict }: { dict: Pick<Dictionary, "proof"> }) {
  return (
    <section aria-label={dict.proof.label} className="py-6">
      <Container>
        <ul className="flex flex-wrap items-center justify-center gap-x-8 gap-y-2 text-sm font-medium text-fg-2">
          {dict.proof.items.map((item) => (
            <li key={item} className="flex items-center gap-8">
              <span>{item}</span>
              <span
                aria-hidden
                className="hidden size-1 rounded-full bg-separator last:hidden sm:block"
              />
            </li>
          ))}
        </ul>
      </Container>
    </section>
  );
}

// ---------------------------------------------------------------------------- como funciona

export function HowItWorks({ dict }: { dict: Pick<Dictionary, "how"> }) {
  const { how } = dict;
  const visuals: ReactNode[] = [
    <WizardArt key="wizard" steps={how.wizardSteps} />,
    <div key="qr" className="flex items-end gap-4">
      <QrArt label={how.qrLabel} />
      <div className="pb-1" aria-hidden>
        <p className="text-xs text-fg-3">{how.pinLabel}</p>
        <p className="mt-1 font-mono text-lg font-semibold tracking-[0.25em] tabular-nums">
          482 913
        </p>
      </div>
    </div>,
    <div key="play" className="w-48" aria-hidden>
      <GamepadArt color="--slot-1" labels />
    </div>,
  ];
  return (
    <section id="how" aria-labelledby="how-title" className="py-24">
      <Container>
        <Reveal>
          <SectionHeading id="how-title" eyebrow={how.eyebrow} title={how.title} />
        </Reveal>
        <ol className="mt-14 grid gap-5 md:grid-cols-3">
          {how.steps.map((step, i) => (
            <li key={step.title}>
              <Reveal delay={i * 0.08} className="h-full">
                <div className="flex h-full flex-col rounded-3xl p-6 glass hairline sm:p-7">
                  <span aria-hidden className="text-sm font-semibold text-accent tabular-nums">
                    {String(i + 1).padStart(2, "0")}
                  </span>
                  <h3 className="mt-2 text-xl font-semibold tracking-tight">{step.title}</h3>
                  <p className="mt-2 text-[15px] text-pretty text-fg-2">{step.body}</p>
                  <div className="mt-auto pt-8">{visuals[i]}</div>
                </div>
              </Reveal>
            </li>
          ))}
        </ol>
      </Container>
    </section>
  );
}

// ---------------------------------------------------------------------------- recursos

function FeatureCard({
  title,
  body,
  children,
  className,
  delay = 0,
}: {
  title: string;
  body: string;
  children?: ReactNode;
  className?: string;
  delay?: number;
}) {
  return (
    <Reveal delay={delay} className={cn("h-full", className)}>
      <article className="flex h-full flex-col rounded-3xl p-6 glass hairline sm:p-7">
        {children && <div className="mb-6">{children}</div>}
        <h3 className="text-xl font-semibold tracking-tight text-balance">{title}</h3>
        <p className="mt-2 text-[15px] text-pretty text-fg-2">{body}</p>
      </article>
    </Reveal>
  );
}

export function Features({ dict }: { dict: Pick<Dictionary, "features" | "hero"> }) {
  const { features: f } = dict;
  return (
    <section id="features" aria-labelledby="features-title" className="py-24">
      <Container>
        <Reveal>
          <SectionHeading id="features-title" eyebrow={f.eyebrow} title={f.title} />
        </Reveal>
        <div className="mt-14 grid gap-4 lg:grid-cols-6">
          <FeatureCard className="lg:col-span-4" title={f.latency.title} body={f.latency.body}>
            <div className="flex flex-wrap items-end gap-x-10 gap-y-3">
              <div>
                <p className="text-5xl font-semibold tracking-tight tabular-nums">0,04 ms</p>
                <p className="mt-1 text-sm text-fg-3">{f.latency.p99}</p>
              </div>
              <div>
                <p className="text-5xl font-semibold tracking-tight text-fg-2 tabular-nums">
                  {"< 10 ms"}
                </p>
                <p className="mt-1 text-sm text-fg-3">{f.latency.goal}</p>
              </div>
            </div>
            <div className="mt-6">
              <LatencyArt label={f.latency.chartLabel} />
            </div>
          </FeatureCard>
          <FeatureCard
            className="lg:col-span-2"
            delay={0.06}
            title={f.players.title}
            body={f.players.body}
          >
            <SlotsArt dict={dict} />
          </FeatureCard>
          <FeatureCard className="lg:col-span-2" title={f.gyro.title} body={f.gyro.body}>
            <GyroArt labels={f.gyro} />
          </FeatureCard>
          <FeatureCard
            className="lg:col-span-2"
            delay={0.06}
            title={f.rumble.title}
            body={f.rumble.body}
          >
            <RumbleArt />
          </FeatureCard>
          <FeatureCard
            className="lg:col-span-2"
            delay={0.12}
            title={f.layout.title}
            body={f.layout.body}
          >
            <LayoutsArt names={f.layout.names} />
          </FeatureCard>
          <Reveal className="lg:col-span-6">
            <article className="grid items-center gap-8 rounded-3xl p-6 glass hairline sm:p-8 md:grid-cols-[1fr_auto]">
              <div className="max-w-xl">
                <h3 className="text-xl font-semibold tracking-tight">{f.privacy.title}</h3>
                <p className="mt-2 text-[15px] text-pretty text-fg-2">{f.privacy.body}</p>
              </div>
              <PrivacyArt />
            </article>
          </Reveal>
        </div>
      </Container>
    </section>
  );
}

// ---------------------------------------------------------------------------- engenharia

function PacketDiagram({ dict }: { dict: Pick<Dictionary, "tech"> }) {
  const { packet } = dict.tech;
  const segments = [
    {
      name: packet.header,
      fields: packet.headerFields,
      bytes: 12,
      tone: "bg-control",
      note: undefined,
    },
    {
      name: packet.payload,
      fields: packet.payloadFields,
      bytes: 24,
      tone: "bg-accent-soft",
      note: packet.encrypted,
    },
    { name: packet.tag, fields: "Poly1305", bytes: 16, tone: "bg-control", note: undefined },
  ];
  return (
    <div className="rounded-3xl p-6 glass hairline sm:p-7">
      <h3 className="mb-5 text-xs font-semibold tracking-wide text-fg-3 uppercase">
        {packet.title}
      </h3>
      <div className="flex flex-col gap-2 sm:flex-row">
        {segments.map((s) => (
          <div
            key={s.name}
            className={cn("min-w-0 rounded-2xl p-4", s.tone)}
            style={{ flex: `${String(s.bytes)} 1 0%` }}
          >
            <p className="text-sm font-semibold">{s.name}</p>
            <p className="mt-1 font-mono text-[11px] leading-snug break-words text-fg-2">
              {s.fields}
            </p>
            <div className="mt-3 flex items-center gap-2">
              <span className="text-xs text-fg-3 tabular-nums">
                {fmt(packet.bytes, { n: s.bytes })}
              </span>
              {s.note && (
                <span className="rounded-full bg-accent px-2 py-0.5 text-[10px] font-semibold text-on-accent">
                  {s.note}
                </span>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

export function Tech({ dict }: { dict: Pick<Dictionary, "tech"> }) {
  const { tech } = dict;
  return (
    <section id="tech" aria-labelledby="tech-title" className="py-24">
      <Container className="grid gap-12 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.05fr)] lg:gap-16">
        <div>
          <Reveal>
            <SectionHeading
              id="tech-title"
              eyebrow={tech.eyebrow}
              title={tech.title}
              lead={tech.lead}
            />
          </Reveal>
          <ul className="mt-10 space-y-7">
            {tech.points.map((point, i) => (
              <li key={point.title}>
                <Reveal delay={i * 0.06}>
                  <div className="flex gap-4">
                    <span aria-hidden className="mt-2.5 size-1.5 shrink-0 rounded-full bg-accent" />
                    <div>
                      <h3 className="font-semibold">{point.title}</h3>
                      <p className="mt-1 text-[15px] text-pretty text-fg-2">{point.body}</p>
                    </div>
                  </div>
                </Reveal>
              </li>
            ))}
          </ul>
        </div>
        <div className="flex flex-col gap-4">
          <Reveal>
            <PacketDiagram dict={dict} />
          </Reveal>
          <Reveal delay={0.08}>
            <div className="rounded-3xl p-6 glass hairline sm:p-7">
              <h3 className="mb-4 text-xs font-semibold tracking-wide text-fg-3 uppercase">
                {tech.table.title}
              </h3>
              <dl className="divide-y divide-[var(--separator)]">
                {tech.table.rows.map((row) => (
                  <div key={row.label} className="flex items-baseline justify-between gap-6 py-3">
                    <dt className="text-[15px] text-fg-2">{row.label}</dt>
                    <dd className="text-lg font-semibold tabular-nums">{row.value}</dd>
                  </div>
                ))}
              </dl>
              <p className="mt-4 text-xs text-pretty text-fg-3">{tech.table.note}</p>
            </div>
          </Reveal>
        </div>
      </Container>
    </section>
  );
}

// ---------------------------------------------------------------------------- FAQ

export function Faq({ dict }: { dict: Pick<Dictionary, "faq"> }) {
  const { faq } = dict;
  return (
    <section id="faq" aria-labelledby="faq-title" className="py-24">
      <Container className="grid gap-12 lg:grid-cols-[minmax(0,0.8fr)_minmax(0,1.2fr)] lg:gap-16">
        <Reveal>
          <SectionHeading id="faq-title" eyebrow={faq.eyebrow} title={faq.title} />
        </Reveal>
        <Reveal delay={0.06}>
          <div className="divide-y divide-[var(--separator)] border-y border-[var(--separator)]">
            {faq.items.map((item) => (
              <details key={item.q} name="faq" className="group">
                <summary className="flex cursor-pointer list-none items-center justify-between gap-6 rounded py-5 text-left text-[17px] font-medium focus-ring [&::-webkit-details-marker]:hidden">
                  {item.q}
                  <span
                    aria-hidden
                    className="relative grid size-7 shrink-0 place-items-center rounded-full bg-control transition-transform duration-300 group-open:rotate-45"
                  >
                    <span className="absolute h-0.5 w-3 rounded bg-fg-2" />
                    <span className="absolute h-3 w-0.5 rounded bg-fg-2" />
                  </span>
                </summary>
                <p className="pb-6 text-[15px] text-pretty text-fg-2">{item.a}</p>
              </details>
            ))}
          </div>
        </Reveal>
      </Container>
    </section>
  );
}
