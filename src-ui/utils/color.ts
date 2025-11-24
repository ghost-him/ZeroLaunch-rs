/**
 * Color utility class to handle color parsing and conversion
 */
class Color {
  r: number;
  g: number;
  b: number;
  a: number;

  constructor(r: number, g: number, b: number, a: number = 1) {
    this.r = Math.max(0, Math.min(255, Math.round(r)));
    this.g = Math.max(0, Math.min(255, Math.round(g)));
    this.b = Math.max(0, Math.min(255, Math.round(b)));
    this.a = Math.max(0, Math.min(1, a));
  }

  static fromString(color: string): Color | null {
    if (!color) return null;
    const c = color.trim();

    // Hex format
    if (c.startsWith('#')) {
      const hex = c.substring(1);
      if (hex.length === 3) {
        // #RGB
        const r = parseInt(hex[0] + hex[0], 16);
        const g = parseInt(hex[1] + hex[1], 16);
        const b = parseInt(hex[2] + hex[2], 16);
        return new Color(r, g, b);
      } else if (hex.length === 6) {
        // #RRGGBB
        const r = parseInt(hex.substring(0, 2), 16);
        const g = parseInt(hex.substring(2, 4), 16);
        const b = parseInt(hex.substring(4, 6), 16);
        return new Color(r, g, b);
      } else if (hex.length === 8) {
        // #RRGGBBAA
        const r = parseInt(hex.substring(0, 2), 16);
        const g = parseInt(hex.substring(2, 4), 16);
        const b = parseInt(hex.substring(4, 6), 16);
        const a = parseInt(hex.substring(6, 8), 16) / 255;
        return new Color(r, g, b, a);
      }
    }

    // RGB/RGBA format
    if (c.toLowerCase().startsWith('rgb')) {
      const match = c.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?\)/i);
      if (match) {
        const r = parseInt(match[1], 10);
        const g = parseInt(match[2], 10);
        const b = parseInt(match[3], 10);
        const a = match[4] !== undefined ? parseFloat(match[4]) : 1;
        return new Color(r, g, b, a);
      }
    }

    return null;
  }

  toHex(includeAlpha: boolean = true): string {
    const toHexPair = (n: number) => n.toString(16).padStart(2, '0');
    let hex = `#${toHexPair(this.r)}${toHexPair(this.g)}${toHexPair(this.b)}`;
    if (includeAlpha) {
      const alphaInt = Math.round(this.a * 255);
      hex += toHexPair(alphaInt);
    }
    return hex;
  }

  toRgba(): string {
    const aStr = parseFloat(this.a.toFixed(3)).toString();
    return `rgba(${this.r}, ${this.g}, ${this.b}, ${aStr})`;
  }
}

export function reduceOpacity(color: string, factor: number = 0.5): string {
  // 验证因子范围
  if (factor < 0 || factor > 1) {
    console.warn('透明度因子必须在 0-1 范围内, 已自动调整');
    factor = Math.max(0, Math.min(1, factor));
  }

  const c = Color.fromString(color);
  if (!c) return color;

  c.a = c.a * factor;
  return c.toHex(true);
}

export function rgbaToHex(color: string): string {
  const c = Color.fromString(color);
  if (!c) return color;
  return c.toHex(true);
}

/**
 * 将任意颜色格式转换为带降低透明度的颜色字符串
 * @param color 原始颜色（支持 rgba、rgb、hex 格式）
 * @param opacityFactor 透明度因子 (0-1)，默认 0.6
 * @returns 处理后的颜色字符串
 */
export function getColorWithReducedOpacity(color: string, opacityFactor: number = 0.6): string {
  return reduceOpacity(color, opacityFactor);
}
