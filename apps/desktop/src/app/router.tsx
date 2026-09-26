import { createHashRouter } from "react-router";
import { RootLayout } from "./RootLayout";
import { HomePage } from "@/features/home/HomePage";

// Hash router: não depende de fallback de rotas no servidor de assets do Tauri.
// Sidebar, rotas das seções e onboarding chegam na T-503.
export const router = createHashRouter([
  {
    path: "/",
    element: <RootLayout />,
    children: [{ index: true, element: <HomePage /> }],
  },
]);
