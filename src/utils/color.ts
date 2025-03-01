function reduceOpacity(color: string, factor: number = 0.5): string {
  // 验证输入颜色格式
  console.log(color)
  if (!/^#[0-9A-Fa-f]{8}$/.test(color)) {
    throw new Error('颜色格式必须为 #RRGGBBAA');
  }

  // 验证因子范围
  if (factor < 0 || factor > 1) {
    throw new Error('透明度因子必须在 0-1 范围内');
  }

  // 提取颜色的 RGB 和 Alpha 部分
  const r = color.substring(1, 3);
  const g = color.substring(3, 5);
  const b = color.substring(5, 7);
  const a = color.substring(7, 9);

  // 将 Alpha 值从十六进制转换为十进制
  const alphaDecimal = parseInt(a, 16);
  
  // 计算新的 Alpha 值（降低透明度）
  const newAlphaDecimal = Math.max(0, Math.floor(alphaDecimal * factor));
  
  // 将新的 Alpha 值转换回十六进制，并确保是两位数
  const newAlphaHex = newAlphaDecimal.toString(16).padStart(2, '0');
  
  // 返回新的 RGBA 颜色
  return `#${r}${g}${b}${newAlphaHex}`;
}

function rgbaToHex(color: string): string {
  // 检查是否以 "rgba(" 开头并以 ")" 结尾
  if (!color.toLowerCase().startsWith('rgba(') || !color.endsWith(')')) {
    return color;
  }
  
  // 提取括号内的内容
  const content = color.substring(5, color.length - 1);
  
  // 分割 RGBA 值
  const parts = content.split(',');
  
  // 确保有 4 个部分 (r, g, b, a)
  if (parts.length !== 4) {
    return color;
  }
  
  // 解析 RGBA 值
  const r = parseInt(parts[0].trim(), 10);
  const g = parseInt(parts[1].trim(), 10);
  const b = parseInt(parts[2].trim(), 10);
  const a = parseFloat(parts[3].trim());
  
  // 检查值是否有效
  if (isNaN(r) || isNaN(g) || isNaN(b) || isNaN(a) ||
      r < 0 || r > 255 || g < 0 || g > 255 || b < 0 || b > 255 || a < 0 || a > 1) {
    return color;
  }
  
  // 将 RGB 转换为 HEX
  const toHex = (value: number): string => {
    const hex = value.toString(16);
    return hex.length === 1 ? '0' + hex : hex;
  };
  
  // 将 Alpha 值（0-1）转换为 HEX（00-FF）
  const alphaHex = Math.round(a * 255).toString(16).padStart(2, '0');
  
  // 返回 HEX 格式 (#RRGGBBAA)
  return `#${toHex(r)}${toHex(g)}${toHex(b)}${alphaHex}`;
}

export { reduceOpacity, rgbaToHex };