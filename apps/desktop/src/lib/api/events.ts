import type { QueryClient } from "@tanstack/react-query";
import { commands, events } from "@/lib/bindings";
import { log } from "@/lib/log";
import { keys } from "./keys";
import { resetLiveSubscriptions, useLiveInput } from "./live";
import { useApprovals } from "./server";
import { useSlots } from "./slots";

let registered: Promise<() => void> | null = null;

/**
 * Liga os eventos do backend ao cache do React Query e aos stores. Roda **uma vez** no boot
 * (chamar de novo, como faz o StrictMode em dev, devolve o mesmo registro) e devolve a função
 * que desfaz tudo.
 */
export function registerEventListeners(queryClient: QueryClient): Promise<() => void> {
  registered ??= register(queryClient);
  return registered;
}

async function register(queryClient: QueryClient): Promise<() => void> {
  const invalidate = (...queryKeys: (readonly unknown[])[]) => {
    for (const queryKey of queryKeys) void queryClient.invalidateQueries({ queryKey });
  };

  const unlisten = await Promise.all([
    events.settingsChanged.listen((e) => {
      queryClient.setQueryData(keys.settings, e.payload.settings);
    }),
    events.slotsChanged.listen((e) => {
      useSlots.getState().setSlots(e.payload.state);
    }),
    events.devicesChanged.listen(() => {
      invalidate(keys.devices);
    }),
    events.devicesPaired.listen(() => {
      invalidate(keys.devices, keys.connectionLog);
    }),
    events.devicesDiscovered.listen((e) => {
      queryClient.setQueryData(keys.nearby, e.payload.devices);
    }),
    events.sessionStarted.listen(() => {
      invalidate(keys.devices, keys.connectionLog);
    }),
    events.sessionEnded.listen((e) => {
      useSlots.getState().clearHealth(e.payload.slot);
      invalidate(keys.devices, keys.connectionLog);
    }),
    events.sessionState.listen((e) => {
      useSlots.getState().setHealth(e.payload.slot, e.payload.health);
    }),
    events.connectionRejected.listen(() => {
      invalidate(keys.connectionLog);
    }),
    events.reservationExpired.listen(() => {
      invalidate(keys.devices);
    }),
    events.approvalRequested.listen((e) => {
      useApprovals.getState().add(e.payload.prompt);
    }),
    events.approvalResolved.listen((e) => {
      useApprovals.getState().remove(e.payload.request_id);
    }),
    events.driverChanged.listen((e) => {
      queryClient.setQueryData(keys.driver, e.payload.status);
    }),
    events.inputLive.listen((e) => {
      useLiveInput.getState().set(e.payload.slot, e.payload.report);
    }),
  ]);

  // Uma janela recarregada (HMR, F5) perdeu as assinaturas locais, mas o backend ainda as conta:
  // zera os dois lados e relê o estado, para a tela nascer completa.
  resetLiveSubscriptions();
  await commands.resetLiveInput().catch((error: unknown) => {
    log.warn(`não consegui zerar o input ao vivo: ${JSON.stringify(error)}`);
  });
  await commands
    .getSlots()
    .then((slots) => {
      useSlots.getState().setSlots(slots);
    })
    .catch((error: unknown) => {
      log.warn(`não consegui ler os slots: ${JSON.stringify(error)}`);
    });

  return () => {
    registered = null;
    for (const stop of unlisten) stop();
  };
}
