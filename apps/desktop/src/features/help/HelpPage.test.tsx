import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { createMemoryRouter, RouterProvider } from "react-router";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { keys } from "@/lib/api/keys";
import { HelpPage } from "./HelpPage";

function setup(initial = "/help", language: "pt-BR" | "en" = "pt-BR") {
  const client = new QueryClient();
  client.setQueryData(keys.settings, { language });
  const router = createMemoryRouter([{ path: "/help", element: <HelpPage /> }], {
    initialEntries: [initial],
  });
  render(
    <QueryClientProvider client={client}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
  return router;
}

beforeEach(() => {
  Element.prototype.scrollIntoView = () => undefined;
});

afterEach(cleanup);

const nav = () => screen.getByRole("navigation", { name: "Documentos da ajuda" });

describe("ajuda", () => {
  it("abre nos primeiros passos, com a lista de documentos", () => {
    setup();
    expect(screen.getByRole("heading", { level: 1, name: "Ajuda" })).toBeTruthy();
    expect(nav().querySelector('button[aria-current="page"]')?.textContent).toBe(
      "Primeiros passos",
    );
    expect(screen.getByRole("heading", { name: "O que você precisa" })).toBeTruthy();
    expect(nav().querySelectorAll("button")).toHaveLength(6);
  });

  it("trocar de documento na lista mostra o texto dele", () => {
    setup();
    fireEvent.click(screen.getByRole("button", { name: "Gyro: mirar mexendo o celular" }));
    expect(screen.getByRole("heading", { name: "Ligar" })).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "O que você precisa" })).toBeNull();
  });

  it("um tópico de erro abre a solução de problemas e rola até a seção", async () => {
    let scrolled = "";
    Element.prototype.scrollIntoView = function (this: Element) {
      scrolled = this.id;
    };
    setup("/help?topic=err-no-network");
    expect(await screen.findByRole("heading", { name: "Sem rede local" })).toBeTruthy();
    expect(screen.getByRole("heading", { level: 1, name: "Solução de problemas" })).toBeTruthy();
    await waitFor(() => {
      expect(scrolled).toBe("err-no-network");
    });
  });

  it("um link interno leva à seção, mesmo em outro documento", async () => {
    const router = setup("/help?doc=parear-um-celular");
    fireEvent.click(screen.getByRole("link", { name: "O celular não encontra o PC" }));
    await waitFor(() => {
      expect(router.state.location.search).toContain("topic=o-celular-nao-encontra-o-pc");
    });
    expect(router.state.location.search).toContain("doc=solucao-de-problemas");
    expect(
      await screen.findByRole("heading", { level: 2, name: "O celular não encontra o PC" }),
    ).toBeTruthy();
  });

  it("segue o idioma do app", () => {
    setup("/help", "en");
    // O texto da ajuda segue o idioma das configurações.
    expect(screen.getByRole("heading", { level: 1, name: "Getting started" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "What you need" })).toBeTruthy();
  });
});
