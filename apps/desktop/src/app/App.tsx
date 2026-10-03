import { useEffect, useState } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { MotionConfig } from "motion/react";
import { RouterProvider } from "react-router";
import { Toaster } from "@/components/feedback/Toaster";
import { TooltipProvider } from "@/components/ui/tooltip";
import { registerEventListeners } from "@/lib/api/events";
import { log } from "@/lib/log";
import { createQueryClient } from "./queryClient";
import { router } from "./router";

export function App() {
  const [queryClient] = useState(createQueryClient);

  // Os eventos do backend são ligados uma vez, no boot, e valem enquanto a janela existir (não há
  // "desligar": o StrictMode monta duas vezes em dev e `registerEventListeners` devolve o mesmo
  // registro; desfazer no cleanup o deixaria surdo depois da segunda montagem).
  useEffect(() => {
    registerEventListeners(queryClient).catch((error: unknown) => {
      log.error(`não consegui registrar os eventos do backend: ${JSON.stringify(error)}`);
    });
  }, [queryClient]);

  return (
    <QueryClientProvider client={queryClient}>
      {/* `reducedMotion="user"`: respeita prefers-reduced-motion (sem deslocamento, só fade). */}
      <MotionConfig reducedMotion="user">
        <TooltipProvider>
          <RouterProvider router={router} />
          <Toaster />
        </TooltipProvider>
      </MotionConfig>
    </QueryClientProvider>
  );
}
