import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { createMemoryRouter, RouterProvider } from "react-router";
import { afterEach, describe, expect, it, vi } from "vitest";
import { describeRouteError } from "./describeRouteError";
import { RouteError } from "./RouteError";

vi.mock("@/lib/log", () => ({ log: { warn: vi.fn(), error: vi.fn(), info: vi.fn() } }));

afterEach(cleanup);

function Broken(): never {
  throw new Error("quebrou de propósito");
}

describe("RouteError", () => {
  it("troca a tela quebrada por uma mensagem amigável e copia os detalhes", async () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", { clipboard: { writeText } });

    const router = createMemoryRouter([
      { path: "/", element: <Broken />, errorElement: <RouteError /> },
    ]);
    render(<RouterProvider router={router} />);

    expect(await screen.findByText("Algo deu errado nesta tela")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Copiar detalhes" }));

    await waitFor(() => {
      expect(writeText).toHaveBeenCalledOnce();
    });
    expect(String(writeText.mock.calls[0]?.[0])).toContain("quebrou de propósito");
    expect(await screen.findByRole("button", { name: "Copiado" })).toBeTruthy();
  });

  it("descreve erros que não são Error", () => {
    expect(describeRouteError({ code: "internal" })).toBe('{"code":"internal"}');
    expect(describeRouteError("texto")).toBe('"texto"');
  });
});
