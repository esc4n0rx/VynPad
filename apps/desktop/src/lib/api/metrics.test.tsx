import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { commands, resetBindingsMock } from "@/test/bindings-mock";
import { useReportVisibility } from "./metrics";

vi.mock("@/lib/bindings", async () => (await import("@/test/bindings-mock")).bindingsMock);
vi.mock("@/lib/log", () => ({ log: { warn: vi.fn(), error: vi.fn(), info: vi.fn() } }));

function Probe() {
  useReportVisibility();
  return null;
}

function setVisibility(state: DocumentVisibilityState) {
  Object.defineProperty(document, "visibilityState", { value: state, configurable: true });
  document.dispatchEvent(new Event("visibilitychange"));
}

beforeEach(() => {
  resetBindingsMock();
  commands.setUiVisible.mockResolvedValue(undefined);
  setVisibility("visible");
  commands.setUiVisible.mockClear();
});

afterEach(cleanup);

describe("useReportVisibility", () => {
  it("avisa o backend ao montar e a cada mudança de visibilidade", () => {
    render(<Probe />);
    expect(commands.setUiVisible).toHaveBeenLastCalledWith(true);
    setVisibility("hidden");
    expect(commands.setUiVisible).toHaveBeenLastCalledWith(false);
    setVisibility("visible");
    expect(commands.setUiVisible).toHaveBeenLastCalledWith(true);
  });

  it("para de ouvir ao desmontar", () => {
    const view = render(<Probe />);
    view.unmount();
    commands.setUiVisible.mockClear();
    setVisibility("hidden");
    expect(commands.setUiVisible).not.toHaveBeenCalled();
  });
});
