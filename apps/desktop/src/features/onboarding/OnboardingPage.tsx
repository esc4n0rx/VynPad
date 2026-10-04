import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { Button } from "@/components/ui/button";
import { useDriverStatus } from "@/lib/api/server";
import { useSettings, useSettingsPatch } from "@/lib/api/settings";
import { useFirewallStatus } from "@/lib/api/system";
import { cn } from "@/lib/utils";
import { ConnectStep, DoneStep, DriverStep, NetworkStep, WelcomeStep } from "./OnboardingSteps";
import {
  STEPS,
  clearStep,
  indexOf,
  isSkippable,
  loadStep,
  nextStep,
  previousStep,
  saveStep,
  type StepId,
} from "./steps";

/** Tela cheia, sem sidebar: leva do "acabei de instalar" ao "estou jogando com o celular". */
export function OnboardingPage() {
  const { t } = useTranslation("onboarding");
  const navigate = useNavigate();
  const { data: settings } = useSettings();
  const { patch } = useSettingsPatch();
  const driverReady = useDriverStatus().data?.state === "ready";
  const firewallOk = useFirewallStatus().data?.ok === true;
  const [step, setStep] = useState<StepId>(loadStep);
  const [paired, setPaired] = useState(false);

  // Se o app fechar no meio, a introdução recomeça de onde parou.
  useEffect(() => {
    saveStep(step);
  }, [step]);

  const finish = () => {
    patch({ onboarding_completed: true });
    clearStep();
    void navigate("/", { replace: true });
  };

  const number = indexOf(step) + 1;
  const stepName = t(`stepper.steps.${step}`);

  // O que o botão principal diz e se o passo já cumpriu o que pedia.
  const primary = (() => {
    switch (step) {
      case "driver":
        return { label: driverReady ? t("nav.next") : t("nav.withoutDriver"), done: driverReady };
      case "network":
        return { label: t("nav.next"), done: firewallOk };
      case "connect":
        return { label: t("nav.next"), done: paired };
      case "done":
        return { label: t("nav.finish"), done: true };
      case "welcome":
        return { label: t("nav.next"), done: true };
    }
  })();
  // Pular só aparece enquanto o passo opcional ainda não foi cumprido.
  const showSkip = isSkippable(step) && !primary.done;

  return (
    <div className="relative flex h-full flex-col overflow-auto">
      <div aria-hidden className="onboarding-bg pointer-events-none absolute inset-0" />

      <nav aria-label={t("stepper.label")} className="relative z-10 flex justify-center px-6 pt-6">
        <ol className="flex items-center gap-2">
          {STEPS.map((id, index) => {
            const current = id === step;
            const past = index < indexOf(step);
            return (
              <li
                key={id}
                className="flex items-center gap-2"
                aria-current={current ? "step" : undefined}
              >
                <span
                  className={cn(
                    "flex size-6 items-center justify-center rounded-full text-caption font-semibold tabular-nums transition-colors",
                    current && "bg-accent text-on-accent",
                    past && "bg-success/20 text-success",
                    !current && !past && "bg-control text-fg-3",
                  )}
                >
                  {index + 1}
                </span>
                <span
                  className={cn(
                    "hidden text-caption min-[760px]:inline",
                    current ? "text-fg" : "text-fg-3",
                  )}
                >
                  {t(`stepper.steps.${id}`)}
                </span>
                {index < STEPS.length - 1 && <span aria-hidden className="h-px w-6 bg-separator" />}
              </li>
            );
          })}
        </ol>
      </nav>
      <p className="sr-only" role="status">
        {t("stepper.current", { n: number, total: STEPS.length, name: stepName })}
      </p>

      <main className="relative z-10 mx-auto flex w-full max-w-2xl flex-1 flex-col justify-center px-6 py-8">
        <div className="flex flex-col gap-6 rounded-xl p-8 glass">
          {!settings ? null : step === "welcome" ? (
            <WelcomeStep settings={settings} patch={patch} />
          ) : step === "driver" ? (
            <DriverStep />
          ) : step === "network" ? (
            <NetworkStep />
          ) : step === "connect" ? (
            <ConnectStep
              onPaired={() => {
                setPaired(true);
              }}
            />
          ) : (
            <DoneStep settings={settings} patch={patch} />
          )}
        </div>
      </main>

      <footer className="relative z-10 flex items-center justify-between gap-3 px-6 pb-6">
        <div className="w-32">
          {step !== "welcome" && (
            <Button
              variant="ghost"
              onClick={() => {
                setStep(previousStep(step));
              }}
            >
              {t("nav.back")}
            </Button>
          )}
        </div>
        <div className="flex items-center gap-2">
          {showSkip && (
            <Button
              variant="ghost"
              onClick={() => {
                setStep(nextStep(step));
              }}
            >
              {t("nav.skip")}
            </Button>
          )}
          <Button
            variant={primary.done ? "primary" : "secondary"}
            size="lg"
            onClick={() => {
              if (step === "done") finish();
              else setStep(nextStep(step));
            }}
          >
            {primary.label}
          </Button>
        </div>
        <div className="w-32" />
      </footer>
    </div>
  );
}
