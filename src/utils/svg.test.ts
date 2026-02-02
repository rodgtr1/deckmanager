import { describe, it, expect } from "vitest";
import { isSvgUrl, colorizeSvgText, svgToDataUrl } from "./svg";

describe("isSvgUrl", () => {
  it("detects .svg extension", () => {
    expect(isSvgUrl("https://example.com/icon.svg")).toBe(true);
    expect(isSvgUrl("https://example.com/icon.SVG")).toBe(true);
    expect(isSvgUrl("/path/to/icon.svg")).toBe(true);
  });

  it("detects Lucide icon URLs", () => {
    expect(isSvgUrl("https://unpkg.com/lucide-static@latest/icons/play.svg")).toBe(true);
    expect(isSvgUrl("https://cdn.example.com/lucide/icons/pause")).toBe(true);
  });

  it("returns false for non-SVG URLs", () => {
    expect(isSvgUrl("https://example.com/icon.png")).toBe(false);
    expect(isSvgUrl("https://example.com/icon.jpg")).toBe(false);
    expect(isSvgUrl("https://example.com/image.webp")).toBe(false);
  });
});

describe("colorizeSvgText", () => {
  it("replaces stroke='currentColor' with the specified color", () => {
    const svg = '<svg><path stroke="currentColor" d="M0 0"/></svg>';
    const result = colorizeSvgText(svg, "#ff0000");
    expect(result).toContain('stroke="#ff0000"');
    expect(result).not.toContain("currentColor");
  });

  it("replaces fill='currentColor' with the specified color", () => {
    const svg = '<svg><circle fill="currentColor"/></svg>';
    const result = colorizeSvgText(svg, "#00ff00");
    expect(result).toContain('fill="#00ff00"');
  });

  it("replaces black stroke colors", () => {
    const svg1 = '<svg><path stroke="#000000"/></svg>';
    const svg2 = '<svg><path stroke="#000"/></svg>';
    const svg3 = '<svg><path stroke="black"/></svg>';

    expect(colorizeSvgText(svg1, "#ffffff")).toContain('stroke="#ffffff"');
    expect(colorizeSvgText(svg2, "#ffffff")).toContain('stroke="#ffffff"');
    expect(colorizeSvgText(svg3, "#ffffff")).toContain('stroke="#ffffff"');
  });

  it("adds stroke to svg element if not present", () => {
    const svg = '<svg viewBox="0 0 24 24"><path d="M0 0"/></svg>';
    const result = colorizeSvgText(svg, "#0000ff");
    expect(result).toContain('<svg stroke="#0000ff"');
  });

  it("does not add duplicate stroke if already present", () => {
    const svg = '<svg><path stroke="currentColor"/></svg>';
    const result = colorizeSvgText(svg, "#ff0000");
    // Should only have one stroke attribute (the replaced one)
    const strokeCount = (result.match(/stroke="/g) || []).length;
    expect(strokeCount).toBe(1);
  });

  it("handles multiple elements", () => {
    const svg = '<svg><path stroke="currentColor"/><circle stroke="#000"/></svg>';
    const result = colorizeSvgText(svg, "#abcdef");
    expect(result).toBe('<svg><path stroke="#abcdef"/><circle stroke="#abcdef"/></svg>');
  });
});

describe("svgToDataUrl", () => {
  it("converts SVG to base64 data URL", () => {
    const svg = '<svg></svg>';
    const result = svgToDataUrl(svg);
    expect(result).toMatch(/^data:image\/svg\+xml;base64,/);
  });

  it("produces valid base64 encoding", () => {
    const svg = '<svg viewBox="0 0 24 24"><path d="M0 0"/></svg>';
    const result = svgToDataUrl(svg);

    // Extract base64 part and decode
    const base64 = result.replace("data:image/svg+xml;base64,", "");
    const decoded = decodeURIComponent(escape(atob(base64)));
    expect(decoded).toBe(svg);
  });

  it("handles special characters", () => {
    const svg = '<svg><text>Hello & "World"</text></svg>';
    const result = svgToDataUrl(svg);
    expect(result).toMatch(/^data:image\/svg\+xml;base64,/);

    // Verify it can be decoded back
    const base64 = result.replace("data:image/svg+xml;base64,", "");
    const decoded = decodeURIComponent(escape(atob(base64)));
    expect(decoded).toBe(svg);
  });
});
