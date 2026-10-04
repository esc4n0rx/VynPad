import type { Ports } from "@/lib/bindings";

export const MIN_PORT = 1024;
export const MAX_PORT = 65535;
const MDNS_PORT = 5353;

export type PortProblem = "invalidRange" | "duplicate" | "mdns";

/** As mesmas regras do backend, para avisar antes de gravar. */
export function portProblem(ports: Ports): PortProblem | null {
  const all = [ports.control, ports.input, ports.discovery];
  if (all.some((p) => !Number.isInteger(p) || p < MIN_PORT || p > MAX_PORT)) return "invalidRange";
  if (new Set(all).size !== 3) return "duplicate";
  if (all.includes(MDNS_PORT)) return "mdns";
  return null;
}
