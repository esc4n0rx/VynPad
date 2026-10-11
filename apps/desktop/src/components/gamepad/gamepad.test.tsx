import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useLiveInput } from "@/lib/api/live";
import type { InputReportDto } from "@/lib/bindings";
import { commands } from "@/test/bindings-mock";
import { BUTTON_MASKS, GAMEPAD_ELEMENTS, NEUTRAL_REPORT, slotColor } from "./elements";
import { Gamepad } from "./Gamepad";
import { GamepadSvg } from "./GamepadSvg";
import { createGamepadRenderer, STICK_TRAVEL, stickOffset } from "./renderer";
import { onFrame } from "./ticker";

vi.mock("@/lib/bindings", async () => (await import("@/test/bindings-mock")).bindingsMock);
vi.mock("@/lib/log", () => ({ log: { warn: vi.fn(), error: vi.fn(), info: vi.fn() } }));

const report = (patch: Partial<InputReportDto>): InputReportDto => ({
  ...NEUTRAL_REPORT,
  ...patch,
});

beforeEach(() => {
  commands.subscribeLiveInput.mockResolvedValue(null);
  commands.unsubscribeLiveInput.mockResolvedValue(undefined);
});

afterEach(() => {
  cleanup();
  useLiveInput.setState({ reports: {} });
  vi.unstubAllGlobals();
});

function mountSvg() {
  const { container } = render(<GamepadSvg />);
  return container.querySelector("svg") as SVGSVGElement;
}

const pressed = (svg: Element, id: string) =>
  svg.querySelector(`[data-el="${id}"]`)?.getAttribute("data-pressed");

describe("desenho", () => {
  it("tem um grupo nomeado para cada elemento do controle", () => {
    const svg = mountSvg();
    for (const id of GAMEPAD_ELEMENTS) {
      expect(svg.querySelector(`[data-el="${id}"]`), id).not.toBeNull();
    }
    expect(svg.querySelectorAll("[data-el]").length).toBe(GAMEPAD_ELEMENTS.length);
  });

  it("vários controles na tela não repetem ids de gradiente", () => {
    const { container } = render(
      <>
        <GamepadSvg />
        <GamepadSvg />
      </>,
    );
    const ids = [...container.querySelectorAll("[id]")].map((e) => e.id);
    expect(new Set(ids).size).toBe(ids.length);
  });
});

describe("renderer", () => {
  it("acende só os botões da máscara", () => {
    const svg = mountSvg();
    const renderer = createGamepadRenderer(svg);
    renderer.apply(report({ buttons: BUTTON_MASKS["btn-a"] | BUTTON_MASKS["dpad-left"] }));
    expect(pressed(svg, "btn-a")).toBe("true");
    expect(pressed(svg, "dpad-left")).toBe("true");
    expect(pressed(svg, "btn-b")).toBe("false");
    renderer.apply(report({ buttons: 0 }));
    expect(pressed(svg, "btn-a")).toBe("false");
  });

  it("cada bit da máscara acende o seu elemento", () => {
    const svg = mountSvg();
    const renderer = createGamepadRenderer(svg);
    for (const [name, mask] of Object.entries(BUTTON_MASKS)) {
      renderer.apply(report({ buttons: mask }));
      expect(pressed(svg, name), name).toBe("true");
      const others = Object.keys(BUTTON_MASKS).filter((n) => n !== name);
      expect(
        others.every((n) => pressed(svg, n) === "false"),
        name,
      ).toBe(true);
    }
  });

  it("enche os gatilhos de 0 a 1", () => {
    const svg = mountSvg();
    const renderer = createGamepadRenderer(svg);
    renderer.apply(report({ lt: 255, rt: 0 }));
    const lt = svg.querySelector<SVGElement>('[data-el="lt"]');
    expect(lt?.style.getPropertyValue("--v")).toBe("1");
    expect(svg.querySelector<SVGElement>('[data-el="rt"]')?.style.getPropertyValue("--v")).toBe(
      "0",
    );
    renderer.apply(report({ lt: 128 }));
    expect(Number(lt?.style.getPropertyValue("--v"))).toBeCloseTo(0.502, 2);
  });

  it("desloca o cap do analógico (Y para cima sobe no desenho)", () => {
    const svg = mountSvg();
    const renderer = createGamepadRenderer(svg);
    renderer.apply(report({ lx: 32767, ly: 32767 }));
    const cap = svg.querySelector('[data-cap="left"]');
    const [dx, dy] = /translate\(([-\d.]+) ([-\d.]+)\)/
      .exec(cap?.getAttribute("transform") ?? "")!
      .slice(1)
      .map(Number) as [number, number];
    expect(dx).toBeGreaterThan(0);
    expect(dy).toBeLessThan(0);
    expect(Math.hypot(dx, dy)).toBeCloseTo(STICK_TRAVEL, 1);
  });

  it("o curso do analógico é circular", () => {
    const { dx, dy } = stickOffset(32767, 32767);
    expect(Math.hypot(dx, dy)).toBeLessThanOrEqual(STICK_TRAVEL + 1e-9);
    expect(stickOffset(0, 0)).toEqual({ dx: 0, dy: -0 });
  });

  it("não mexe no DOM quando nada mudou", () => {
    const svg = mountSvg();
    const renderer = createGamepadRenderer(svg);
    const same = report({ buttons: BUTTON_MASKS["btn-x"], lx: 1000 });
    renderer.apply(same);
    const spy = vi.spyOn(Element.prototype, "setAttribute");
    renderer.apply(same);
    renderer.apply({ ...same });
    expect(spy).not.toHaveBeenCalled();
  });

  it("sem relatório, solta tudo", () => {
    const svg = mountSvg();
    const renderer = createGamepadRenderer(svg);
    renderer.apply(report({ buttons: BUTTON_MASKS.start, lt: 255 }));
    renderer.apply(undefined);
    expect(pressed(svg, "start")).toBe("false");
  });
});

describe("Gamepad", () => {
  it("modo ao vivo: segue o store e assina o backend só enquanto montado", () => {
    const raf = vi.fn<(cb: FrameRequestCallback) => number>().mockReturnValue(1);
    vi.stubGlobal("requestAnimationFrame", raf);
    vi.stubGlobal("cancelAnimationFrame", vi.fn());

    const { container, unmount } = render(<Gamepad slot={2} />);
    expect(commands.subscribeLiveInput).toHaveBeenCalledWith(2);

    useLiveInput.getState().set(2, report({ buttons: BUTTON_MASKS["btn-y"] }));
    // O laço de animação roda um quadro.
    raf.mock.calls[0]?.[0](0);
    expect(pressed(container, "btn-y")).toBe("true");

    unmount();
    expect(commands.unsubscribeLiveInput).toHaveBeenCalledWith(2);
  });

  it("com `report` fixo não assina o backend", () => {
    const { container } = render(<Gamepad report={report({ buttons: BUTTON_MASKS.guide })} />);
    expect(commands.subscribeLiveInput).not.toHaveBeenCalled();
    expect(pressed(container, "guide")).toBe("true");
  });

  it("modo interativo: todos os elementos são selecionáveis, com nome", () => {
    const onSelect = vi.fn();
    render(<Gamepad mode="interactive" onSelect={onSelect} />);
    const buttons = screen.getAllByRole("button");
    expect(buttons).toHaveLength(GAMEPAD_ELEMENTS.length);
    for (const button of buttons) {
      expect(button.getAttribute("aria-label")).toBeTruthy();
    }

    fireEvent.click(screen.getByRole("button", { name: "A" }));
    expect(onSelect).toHaveBeenLastCalledWith("btn-a");
    fireEvent.keyDown(screen.getByRole("button", { name: "LT (gatilho esquerdo)" }), {
      key: "Enter",
    });
    expect(onSelect).toHaveBeenLastCalledWith("lt");
  });

  it("clicar no cap seleciona o clique do analógico, não o analógico", () => {
    const onSelect = vi.fn();
    render(<Gamepad mode="interactive" onSelect={onSelect} />);
    fireEvent.click(screen.getByRole("button", { name: "Clique do analógico esquerdo (L3)" }));
    expect(onSelect).toHaveBeenCalledOnce();
    expect(onSelect).toHaveBeenCalledWith("ls-click");
  });

  it("marca o elemento selecionado", () => {
    render(<Gamepad mode="interactive" selected="btn-b" onSelect={vi.fn()} />);
    expect(screen.getByRole("button", { name: "B" }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByRole("button", { name: "A" }).getAttribute("aria-pressed")).toBe("false");
  });

  it("usa a cor do slot", () => {
    const { container } = render(<Gamepad report={NEUTRAL_REPORT} color={slotColor(1)} />);
    expect((container.firstElementChild as HTMLElement).style.getPropertyValue("--pad-color")).toBe(
      "var(--slot-2)",
    );
  });
});

describe("ticker", () => {
  it("um único laço serve vários inscritos e para quando não sobra nenhum", () => {
    const raf = vi.fn<(cb: FrameRequestCallback) => number>().mockReturnValue(7);
    const cancel = vi.fn();
    vi.stubGlobal("requestAnimationFrame", raf);
    vi.stubGlobal("cancelAnimationFrame", cancel);

    const a = vi.fn();
    const b = vi.fn();
    const offA = onFrame(a);
    const offB = onFrame(b);
    expect(raf).toHaveBeenCalledTimes(1);

    raf.mock.calls[0]?.[0](0);
    expect(a).toHaveBeenCalledOnce();
    expect(b).toHaveBeenCalledOnce();
    expect(raf).toHaveBeenCalledTimes(2);

    offA();
    offB();
    expect(cancel).toHaveBeenCalled();
  });
});

describe("controle vivo", () => {
  it("só marca como vivo quando pedido e expõe o slot para o anel de LED", () => {
    const { container, rerender } = render(<Gamepad mode="mini" slot={2} report={undefined} />);
    const wrapper = container.querySelector(".gamepad") as HTMLElement;
    expect(wrapper.hasAttribute("data-alive")).toBe(false);
    rerender(<Gamepad mode="mini" slot={2} report={undefined} alive />);
    expect(wrapper.getAttribute("data-alive")).toBe("true");
    expect(wrapper.getAttribute("data-slot")).toBe("2");
    expect(wrapper.querySelectorAll(".pad-led path")).toHaveLength(4);
  });
});
