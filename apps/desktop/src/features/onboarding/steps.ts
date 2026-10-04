export const STEPS = ["welcome", "driver", "network", "connect", "done"] as const;
export type StepId = (typeof STEPS)[number];

/** Passos que dá para pular por enquanto (o do driver tem o seu próprio "continuar sem"). */
const SKIPPABLE: ReadonlySet<StepId> = new Set(["network", "connect"]);

export const STORAGE_KEY = "padgaming.onboarding.step";

export const isSkippable = (step: StepId) => SKIPPABLE.has(step);

export const indexOf = (step: StepId) => STEPS.indexOf(step);

export function nextStep(step: StepId): StepId {
  return STEPS[Math.min(indexOf(step) + 1, STEPS.length - 1)] ?? "done";
}

export function previousStep(step: StepId): StepId {
  return STEPS[Math.max(indexOf(step) - 1, 0)] ?? "welcome";
}

/** Um valor lido do armazenamento só vale se for um passo que existe. */
export function parseStep(raw: unknown): StepId {
  return STEPS.find((s) => s === raw) ?? "welcome";
}

/** Onde o usuário parou (o armazenamento pode falhar: então recomeça do início). */
export function loadStep(): StepId {
  try {
    return parseStep(window.localStorage.getItem(STORAGE_KEY));
  } catch {
    return "welcome";
  }
}

export function saveStep(step: StepId): void {
  try {
    window.localStorage.setItem(STORAGE_KEY, step);
  } catch {
    // Sem armazenamento, só não retoma de onde parou.
  }
}

export function clearStep(): void {
  try {
    window.localStorage.removeItem(STORAGE_KEY);
  } catch {
    // idem
  }
}
