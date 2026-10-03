import { useState } from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Badge } from "./badge";
import { Button, IconButton } from "./button";
import { SegmentedControl } from "./segmented-control";
import { Slider } from "./slider";
import { Stepper } from "./stepper";
import { Toggle } from "./toggle";
import { applyAccent, applyTheme, DEFAULT_ACCENT } from "@/lib/theme";

// O jsdom não implementa ResizeObserver, que o Slider do Radix usa para medir o thumb.
vi.stubGlobal(
  "ResizeObserver",
  class {
    observe() {}
    unobserve() {}
    disconnect() {}
  },
);

afterEach(() => {
  document.documentElement.removeAttribute("data-theme");
  document.documentElement.style.removeProperty("--accent");
});

describe("Button", () => {
  it("é type=button por padrão e dispara onClick", () => {
    const onClick = vi.fn();
    render(<Button onClick={onClick}>Salvar</Button>);
    const button = screen.getByRole("button", { name: "Salvar" });
    expect(button).toHaveProperty("type", "button");
    fireEvent.click(button);
    expect(onClick).toHaveBeenCalledOnce();
  });

  it("IconButton expõe o label como nome acessível", () => {
    render(
      <IconButton label="Configurações">
        <svg />
      </IconButton>,
    );
    expect(screen.getByRole("button", { name: "Configurações" })).toBeTruthy();
  });
});

describe("Toggle", () => {
  it("alterna o estado do switch", () => {
    function Harness() {
      const [on, setOn] = useState(false);
      return <Toggle aria-label="Ativar" checked={on} onCheckedChange={setOn} />;
    }
    render(<Harness />);
    const toggle = screen.getByRole("switch", { name: "Ativar" });
    expect(toggle.getAttribute("aria-checked")).toBe("false");
    fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-checked")).toBe("true");
  });
});

describe("SegmentedControl", () => {
  it("marca a opção atual e notifica a escolha", () => {
    const onValueChange = vi.fn();
    render(
      <SegmentedControl
        label="Tema"
        value="dark"
        onValueChange={onValueChange}
        options={[
          { value: "dark", label: "Dark" },
          { value: "light", label: "Light" },
        ]}
      />,
    );
    expect(screen.getByRole("radio", { name: "Dark" }).getAttribute("aria-checked")).toBe("true");
    fireEvent.click(screen.getByRole("radio", { name: "Light" }));
    expect(onValueChange).toHaveBeenCalledWith("light");
  });
});

describe("Slider", () => {
  it("mostra o valor formatado e a marca do padrão", () => {
    render(
      <Slider
        label="Zona morta"
        value={30}
        onValueChange={vi.fn()}
        min={0}
        max={50}
        defaultValue={10}
        format={(v) => `${v}%`}
      />,
    );
    expect(screen.getByRole("slider", { name: "Zona morta" })).toBeTruthy();
    expect(screen.getByText("30%")).toBeTruthy();
    expect(screen.getByTestId("slider-default-mark").style.left).toBe("20%");
  });

  it("duplo clique no valor volta ao padrão", () => {
    const onValueChange = vi.fn();
    render(
      <Slider
        label="Zona morta"
        value={30}
        onValueChange={onValueChange}
        min={0}
        max={50}
        defaultValue={10}
      />,
    );
    fireEvent.doubleClick(screen.getByText("30"));
    expect(onValueChange).toHaveBeenCalledWith(10);
  });
});

describe("Badge e Stepper", () => {
  it("Badge mostra o texto do estado", () => {
    render(<Badge status="success">Conectado</Badge>);
    expect(screen.getByText("Conectado")).toBeTruthy();
  });

  it("Stepper marca o passo atual", () => {
    render(<Stepper steps={["A", "B", "C"]} current={1} />);
    const current = screen.getByText("B").closest("li");
    expect(current?.getAttribute("aria-current")).toBe("step");
    expect(screen.getByText("A").closest("li")?.getAttribute("aria-current")).toBeNull();
  });
});

describe("tema", () => {
  it("applyTheme grava data-theme no <html>", () => {
    applyTheme("light");
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("applyAccent aplica a cor e cai no padrão se for inválida", () => {
    expect(applyAccent("#30D158")).toBe("#30D158");
    expect(document.documentElement.style.getPropertyValue("--accent")).toBe("#30D158");
    expect(applyAccent("azul")).toBe(DEFAULT_ACCENT);
    expect(document.documentElement.style.getPropertyValue("--accent")).toBe(DEFAULT_ACCENT);
  });
});
