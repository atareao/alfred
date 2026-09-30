import { describe, it, expect } from "vitest";
import indexHtml from "../../index.html?raw";
import manifestRaw from "../../public/manifest.webmanifest?raw";

const publicFiles = import.meta.glob("../../public/*");

interface ManifestIcon {
  src: string;
  sizes: string;
  type: string;
  purpose?: string;
}

interface WebManifest {
  name?: string;
  short_name?: string;
  start_url?: string;
  display?: string;
  theme_color?: string;
  background_color?: string;
  icons?: ManifestIcon[];
}

function hasLink(
  html: string,
  rel: string,
  hrefFragment: string,
  expectedAttrs: Record<string, string> = {},
): boolean {
  const linkTags = html.match(/<link\b[^>]*>/gi) ?? [];
  return linkTags.some((tag) => {
    const relMatch = tag.match(/rel\s*=\s*["']([^"']+)["']/i);
    const hrefMatch = tag.match(/href\s*=\s*["']([^"']+)["']/i);
    if (relMatch?.[1] !== rel || hrefMatch?.[1] === undefined) {
      return false;
    }
    const href = hrefMatch[1];
    const basename = href.split("/").pop();
    if (basename !== hrefFragment && !href.endsWith("/" + hrefFragment)) {
      return false;
    }
    return Object.entries(expectedAttrs).every(([attr, value]) => {
      const attrMatch = tag.match(
        new RegExp(`\\b${attr}\\s*=\\s*["']([^"']+)["']`, "i"),
      );
      return attrMatch?.[1] === value;
    });
  });
}

function publicFileNames(): string[] {
  return Object.keys(publicFiles).map((key) => key.split("/").pop() ?? "");
}

describe("frontend/index.html", () => {
  it("referencia el favicon .ico con sizes=any", () => {
    expect(hasLink(indexHtml, "icon", "favicon.ico", { sizes: "any" })).toBe(
      true,
    );
  });

  it("referencia el favicon de 32x32 con type y sizes", () => {
    expect(
      hasLink(indexHtml, "icon", "favicon-32x32.png", {
        type: "image/png",
        sizes: "32x32",
      }),
    ).toBe(true);
  });

  it("referencia el favicon de 16x16 con type y sizes", () => {
    expect(
      hasLink(indexHtml, "icon", "favicon-16x16.png", {
        type: "image/png",
        sizes: "16x16",
      }),
    ).toBe(true);
  });

  it("referencia el apple-touch-icon", () => {
    expect(hasLink(indexHtml, "apple-touch-icon", "apple-touch-icon.png")).toBe(
      true,
    );
  });

  it("referencia el manifest PWA", () => {
    expect(hasLink(indexHtml, "manifest", "manifest.webmanifest")).toBe(true);
  });

  it("no referencia vite.svg", () => {
    expect(indexHtml).not.toContain("vite.svg");
  });
});

describe("frontend/public/manifest.webmanifest", () => {
  it("es JSON válido con los campos requeridos", () => {
    const manifest = JSON.parse(manifestRaw) as WebManifest;
    expect(manifest.name).toBeTruthy();
    expect(manifest.short_name).toBeTruthy();
    expect(manifest.start_url).toBeTruthy();
    expect(manifest.display).toBe("standalone");
    expect(manifest.theme_color).toBeTruthy();
    expect(manifest.background_color).toBeTruthy();
    expect(Array.isArray(manifest.icons)).toBe(true);
  });

  it("declara los iconos 192x192 y 512x512 de tipo image/png", () => {
    const manifest = JSON.parse(manifestRaw) as WebManifest;
    const icons = manifest.icons ?? [];
    expect(
      icons.some((icon) => icon.sizes === "192x192" && icon.type === "image/png"),
    ).toBe(true);
    expect(
      icons.some((icon) => icon.sizes === "512x512" && icon.type === "image/png"),
    ).toBe(true);
  });

  it("incluye al menos un icono maskable", () => {
    const manifest = JSON.parse(manifestRaw) as WebManifest;
    const icons = manifest.icons ?? [];
    expect(
      icons.some((icon) => icon.purpose?.split(/\s+/).includes("maskable")),
    ).toBe(true);
  });
});

describe("frontend/public/", () => {
  const expectedFiles = [
    "favicon.ico",
    "favicon-16x16.png",
    "favicon-32x32.png",
    "favicon-48x48.png",
    "apple-touch-icon.png",
    "icon-192.png",
    "icon-512.png",
  ];

  it.each(expectedFiles)("contiene %s", (file) => {
    expect(publicFileNames()).toContain(file);
  });
});
