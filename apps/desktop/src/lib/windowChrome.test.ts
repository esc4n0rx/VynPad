import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands } from "@/test/bindings-mock";
import { applyWindowChrome, bootWindowChrome, FALLBACK_CHROME } from "./windowChrome";

vi.mock("@/lib/bindings", async () => (await import("@/test/bindings-mock")).bindingsMock);
vi.mock("@/lib/log", () => ({ log: { warn: vi.fn(), error: vi.fn(), info: vi.fn() } }));

beforeEach(() => {
  delete document.documentElement.dataset.material;
  delete document.documentElement.dataset.corners;
});

describe("windowChrome", () => {
  it("applyWindowChrome grava material e cantos no <html>", () => {
    applyWindowChrome({ material: "mica", corners: "native" });
    expect(document.documentElement.dataset.material).toBe("mica");
    expect(document.documentElement.dataset.corners).toBe("native");
  });

  it("bootWindowChrome aplica o que o backend informou", async () => {
    commands.windowChrome.mockResolvedValue({ material: "acrylic", corners: "square" });
    await expect(bootWindowChrome()).resolves.toEqual({ material: "acrylic", corners: "square" });
    expect(document.documentElement.dataset.material).toBe("acrylic");
  });

  it("se o backend falhar, cai para sólido com cantos em CSS", async () => {
    commands.windowChrome.mockRejectedValue(new Error("sem backend"));
    await expect(bootWindowChrome()).resolves.toEqual(FALLBACK_CHROME);
    expect(document.documentElement.dataset.material).toBe("solid");
    expect(document.documentElement.dataset.corners).toBe("css");
  });
});
