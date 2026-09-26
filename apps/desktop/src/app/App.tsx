import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "react-router";
import { router } from "./router";

const queryClient = new QueryClient({
  defaultOptions: {
    // Dados vêm do backend local: sem refetch agressivo por foco de janela.
    queries: { refetchOnWindowFocus: false, retry: 1 },
  },
});

export function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  );
}
