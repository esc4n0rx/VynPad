import { describe, expect, it } from "vitest";
import { detectOs } from "./os";

const UA = {
  win: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/130 Safari/537.36",
  android:
    "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 Chrome/130 Mobile Safari/537.36",
  iphone:
    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) AppleWebKit/605.1.15 Mobile/15E148",
  mac: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 Safari/605.1.15",
  linux: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/130 Safari/537.36",
};

describe("detectOs", () => {
  it("reconhece os sistemas pelo userAgent", () => {
    expect(detectOs(UA.win)).toBe("windows");
    expect(detectOs(UA.android)).toBe("android");
    expect(detectOs(UA.iphone)).toBe("ios");
    expect(detectOs(UA.mac)).toBe("macos");
    expect(detectOs(UA.linux)).toBe("linux");
    expect(detectOs("curl/8.0")).toBe("other");
  });

  it("Android não vira Linux, mesmo o userAgent dizendo Linux", () => {
    expect(detectOs(UA.android)).toBe("android");
  });

  it("a dica de plataforma do navegador vale sobre o userAgent", () => {
    expect(detectOs("Mozilla/5.0", "Windows")).toBe("windows");
    expect(detectOs("Mozilla/5.0", "Android")).toBe("android");
    expect(detectOs("Mozilla/5.0", "macOS")).toBe("macos");
  });
});
