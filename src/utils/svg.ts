// SVG utility functions for icon colorization

/**
 * Detect if a URL points to an SVG file
 */
export const isSvgUrl = (url: string): boolean => {
  return url.toLowerCase().endsWith(".svg") || (url.includes("/icons/") && url.includes("lucide"));
};

/**
 * Colorize an SVG by replacing stroke colors
 * @param svgText - The raw SVG text content
 * @param color - The color to apply (hex format, e.g., "#ffffff")
 * @returns The colorized SVG text
 */
export const colorizeSvgText = (svgText: string, color: string): string => {
  let result = svgText;

  // Replace stroke and fill colors
  // Lucide icons use stroke="currentColor"
  result = result.replace(/stroke="currentColor"/g, `stroke="${color}"`);
  result = result.replace(/fill="currentColor"/g, `fill="${color}"`);
  // Also handle icons that might use black
  result = result.replace(/stroke="#000000"/g, `stroke="${color}"`);
  result = result.replace(/stroke="#000"/g, `stroke="${color}"`);
  result = result.replace(/stroke="black"/g, `stroke="${color}"`);
  // For icons without explicit stroke color, add it to the svg element
  if (!result.includes('stroke="')) {
    result = result.replace(/<svg/, `<svg stroke="${color}"`);
  }

  return result;
};

/**
 * Convert SVG text to a data URL
 */
export const svgToDataUrl = (svgText: string): string => {
  const base64 = btoa(unescape(encodeURIComponent(svgText)));
  return `data:image/svg+xml;base64,${base64}`;
};

/**
 * Fetch an SVG from URL, colorize it, and return as a data URL
 */
export const colorizeSvg = async (url: string, color: string): Promise<string> => {
  try {
    const response = await fetch(url);
    if (!response.ok) throw new Error("Failed to fetch SVG");
    const svgText = await response.text();
    const colorized = colorizeSvgText(svgText, color);
    return svgToDataUrl(colorized);
  } catch (e) {
    console.error("Failed to colorize SVG:", e);
    return url; // Return original URL on failure
  }
};
