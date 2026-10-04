import type { ReactNode } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, waitFor } from "@testing-library/react";
import { createMemoryRouter, RouterProvider } from "react-router";
import { afterEach, describe, expect, it } from "vitest";
import { keys } from "./api/keys";
import type { Settings } from "./api/settings";
import { useOnboardingRedirect } from "./useOnboardingRedirect";

function Probe() {
  useOnboardingRedirect();
  return null;
}

function setup(settings: Partial<Settings> | null, initial = "/") {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  if (settings) client.setQueryData(keys.settings, { onboarding_completed: false, ...settings });
  const router = createMemoryRouter(
    [
      { path: "/", element: <Probe /> },
      { path: "/onboarding", element: <Probe /> },
      { path: "/devices", element: <Probe /> },
    ],
    { initialEntries: [initial] },
  );
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  render(<RouterProvider router={router} />, { wrapper });
  return { router, client };
}

afterEach(cleanup);

describe("useOnboardingRedirect", () => {
  it("leva à introdução quando ela ainda não foi concluída", async () => {
    const { router } = setup({ onboarding_completed: false });
    await waitFor(() => {
      expect(router.state.location.pathname).toBe("/onboarding");
    });
  });

  it("não mexe em quem já concluiu", async () => {
    const { router } = setup({ onboarding_completed: true });
    await Promise.resolve();
    expect(router.state.location.pathname).toBe("/");
  });

  it("só decide uma vez: navegar depois não é interrompido", async () => {
    const { router, client } = setup({ onboarding_completed: false });
    await waitFor(() => {
      expect(router.state.location.pathname).toBe("/onboarding");
    });
    await router.navigate("/devices");
    client.setQueryData(keys.settings, { onboarding_completed: false });
    await Promise.resolve();
    expect(router.state.location.pathname).toBe("/devices");
  });

  it("espera as configurações chegarem", async () => {
    const { router, client } = setup(null);
    await Promise.resolve();
    expect(router.state.location.pathname).toBe("/");
    client.setQueryData(keys.settings, { onboarding_completed: false });
    await waitFor(() => {
      expect(router.state.location.pathname).toBe("/onboarding");
    });
  });
});
