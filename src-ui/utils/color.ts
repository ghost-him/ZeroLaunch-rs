export function reduceOpacity(color: string, factor: number = 0.5): string {
  // 验证输入颜色格式

  if (!/^#[0-9A-Fa-f]{8}$/.test(color)) {
    // throw new Error('颜色格式必须为 #RRGGBBAA');
    return color; // Fail gracefully or return original if not matching
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

export function rgbaToHex(color: string): string {
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
  
  // 将 RGB 值转换为十六进制
  const rHex = r.toString(16).padStart(2, '0');
  const gHex = g.toString(16).padStart(2, '0');
  const bHex = b.toString(16).padStart(2, '0');
  
  // 将 Alpha 值转换为十六进制 (0-255)
  const aHex = Math.round(a * 255).toString(16).padStart(2, '0');
  
  // 返回十六进制颜色字符串
  return `#${rHex}${gHex}${bHex}${aHex}`;
}
