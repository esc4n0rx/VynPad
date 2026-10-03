/**
 * Um único laço de `requestAnimationFrame` para todos os controles na tela: com 4 gamepads é um
 * agendamento por quadro, não quatro. O laço só roda enquanto alguém está inscrito.
 */
const callbacks = new Set<() => void>();
let handle: number | undefined;

function tick(): void {
  for (const callback of callbacks) callback();
  handle = callbacks.size > 0 ? requestAnimationFrame(tick) : undefined;
}

/** Chama `callback` a cada quadro até o `unsubscribe` devolvido. */
export function onFrame(callback: () => void): () => void {
  callbacks.add(callback);
  handle ??= requestAnimationFrame(tick);
  return () => {
    callbacks.delete(callback);
    if (callbacks.size === 0 && handle !== undefined) {
      cancelAnimationFrame(handle);
      handle = undefined;
    }
  };
}
