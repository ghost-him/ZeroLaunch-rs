export type RGB = { r: number; g: number; b: number };

/**
 * 将十六进制颜色转换为 RGB
 */
export function hexToRgb(hex: string): RGB {
  hex = hex.replace('#', '');
  if (hex.length === 3) {
    hex = hex.split('').map((char) => char + char).join('');
  }
  if (hex.length !== 6) {
    throw new Error('Invalid hex color format');
  }

  const bigint = parseInt(hex, 16);
  return {
    r: (bigint >> 16) & 255,
    g: (bigint >> 8) & 255,
    b: bigint & 255,
  };
}

/**
 * 将 RGB 转换为 RGBA 字符串
 */
export function rgbToRgba(rgb: RGB, alpha: number): string {
  return `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${alpha})`;
}

/**
 * 调整 RGB 的亮度
 */
export function adjustBrightness(rgb: RGB, factor: number): RGB {
  return {
    r: Math.min(255, Math.max(0, Math.floor(rgb.r + (255 - rgb.r) * factor))),
    g: Math.min(255, Math.max(0, Math.floor(rgb.g + (255 - rgb.g) * factor))),
    b: Math.min(255, Math.max(0, Math.floor(rgb.b + (255 - rgb.b) * factor))),
  };
}

/**
 * 计算选中和非选中项的颜色
 */
export function calculateColors(hex: string): { selected: string; nonSelected: string } {
  const selectedItemRgb = hexToRgb(hex);
  const selectedItemColor = rgbToRgba(selectedItemRgb, 0.8);

  const nonSelectedItemRgb = adjustBrightness(selectedItemRgb, 0.3); // 调亮 20%
  const nonSelectedItemColor = rgbToRgba(nonSelectedItemRgb, 0.8);

  return { selected: selectedItemColor, nonSelected: nonSelectedItemColor };
}
// 示例
// const colors = calculateColors('#3498db');
// console.log('Selected Item Color:', colors.selected);
// console.log('Non-Selected Item Color:', colors.nonSelected);