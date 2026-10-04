import type { ComponentType } from "react";
import { createHashRouter, type RouteObject } from "react-router";
import { RouteError } from "@/components/shell/RouteError";
import { RootLayout } from "./RootLayout";
import { ShellLayout } from "./ShellLayout";

/** Rota carregada sob demanda: o código da tela só vem quando ela é aberta. */
function lazyPage<K extends string>(
  load: () => Promise<Record<K, ComponentType>>,
  name: K,
): Pick<RouteObject, "lazy"> {
  return { lazy: async () => ({ Component: (await load())[name] }) };
}

// Hash router: não depende de fallback de rotas no servidor de assets do Tauri.
export const routes: RouteObject[] = [
  {
    path: "/",
    element: <RootLayout />,
    errorElement: <RouteError />,
    children: [
      {
        // Seções com sidebar. O `errorElement` do filho mantém a sidebar de pé se uma tela quebrar.
        element: <ShellLayout />,
        children: [
          {
            errorElement: <RouteError />,
            children: [
              {
                index: true,
                ...lazyPage(() => import("@/features/dashboard/DashboardPage"), "DashboardPage"),
              },
              {
                path: "devices",
                ...lazyPage(() => import("@/features/devices/DevicesPage"), "DevicesPage"),
              },
              {
                path: "mapping",
                ...lazyPage(() => import("@/features/mapping/MappingPage"), "MappingPage"),
              },
              {
                path: "latency",
                ...lazyPage(() => import("@/features/latency/LatencyPage"), "LatencyPage"),
              },
              {
                path: "settings",
                ...lazyPage(() => import("@/features/settings/SettingsPage"), "SettingsPage"),
              },
              // Vitrine dos componentes: só existe em dev (some do bundle de produção).
              ...(import.meta.env.DEV
                ? [
                    {
                      path: "dev/ui",
                      ...lazyPage(() => import("@/features/dev/UiGallery"), "UiGallery"),
                    },
                  ]
                : []),
            ],
          },
        ],
      },
      {
        element: <ShellLayout />,
        children: [
          {
            errorElement: <RouteError />,
            children: [
              {
                path: "help",
                ...lazyPage(() => import("@/features/help/HelpPage"), "HelpPage"),
              },
            ],
          },
        ],
      },
      // Tela cheia, sem sidebar.
      {
        path: "onboarding",
        errorElement: <RouteError />,
        ...lazyPage(() => import("@/features/onboarding/OnboardingPage"), "OnboardingPage"),
      },
    ],
  },
];

export const router = createHashRouter(routes);
