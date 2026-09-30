/**
 * Dublê de `@/lib/bindings` para os testes: cada command é um `vi.fn()` e cada evento guarda o
 * handler registrado, para o teste "emitir" o evento como o backend faria.
 *
 * Uso: `vi.mock("@/lib/bindings", async () => (await import("@/test/bindings-mock")).bindingsMock)`.
 */
import { vi, type Mock } from "vitest";
import type * as Bindings from "@/lib/bindings";

// Os nomes vêm do arquivo gerado: um command ou evento novo em Rust aparece aqui sozinho.
const real = await vi.importActual<typeof Bindings>("@/lib/bindings");

export type CommandMocks = Record<keyof typeof real.commands, Mock>;
export type EventName = keyof typeof real.events;

export const commands = Object.fromEntries(
  Object.keys(real.commands).map((name) => [name, vi.fn()]),
) as unknown as CommandMocks;

type Handler = (event: { payload: unknown }) => void;
const handlers = new Map<string, Handler>();
/** Eventos cujo `unlisten` já foi chamado. */
export const stopped = new Set<string>();

const listeners = Object.fromEntries(
  Object.keys(real.events).map((name) => [
    name,
    {
      listen: vi.fn((handler: Handler) => {
        handlers.set(name, handler);
        return Promise.resolve(() => {
          stopped.add(name);
          handlers.delete(name);
        });
      }),
    },
  ]),
) as unknown as Record<EventName, { listen: Mock }>;

/** Entrega um evento ao handler registrado (como o Tauri faria). */
export function emit(name: EventName, payload: unknown = {}): void {
  const handler = handlers.get(name);
  if (!handler) throw new Error(`nenhum listener registrado para ${name}`);
  handler({ payload });
}

/** Quantos eventos têm listener registrado agora. */
export function listenerCount(): number {
  return handlers.size;
}

/** Limpa o estado entre os testes: commands devolvem `null` e nenhum evento está registrado. */
export function resetBindingsMock(): void {
  handlers.clear();
  stopped.clear();
  for (const command of Object.values(commands)) {
    command.mockReset();
    command.mockResolvedValue(null);
  }
  for (const event of Object.values(listeners)) {
    event.listen.mockClear();
  }
}

export const bindingsMock = { ...real, commands, events: listeners };
