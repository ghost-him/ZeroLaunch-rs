/**
 * 语义化版本比较（仅覆盖插件清单实际使用的形态）。
 *
 * 支持 `1.2.3`、`v1.2.3`、`1.2`、`1`，以及可选预发布后缀 `-beta.1`。
 * 按 semver 规则：数值段逐位比较，缺位视为 0；有预发布后缀的版本低于同号正式版；
 * 两段都有后缀时逐段比较（数值标识按数值比、其余按字典序）。
 *
 * 参数：a - 左版本字符串；b - 右版本字符串。
 * 返回：a < b 返回负数，相等返回 0，a > b 返回正数；任一侧无法解析时返回 null
 *      （调用方据此退化为"不显示版本对比"，不要猜）。
 */
export function compareVersions(a: string, b: string): number | null {
  const left = parseVersion(a)
  const right = parseVersion(b)
  if (!left || !right) return null

  for (let i = 0; i < 3; i++) {
    const diff = left.numeric[i] - right.numeric[i]
    if (diff !== 0) return diff
  }

  // 正式版高于同号预发布版
  if (left.pre.length === 0 && right.pre.length === 0) return 0
  if (left.pre.length === 0) return 1
  if (right.pre.length === 0) return -1

  const len = Math.max(left.pre.length, right.pre.length)
  for (let i = 0; i < len; i++) {
    const l = left.pre[i]
    const r = right.pre[i]
    // 段数少的更小（1.0.0-alpha < 1.0.0-alpha.1）
    if (l === undefined) return -1
    if (r === undefined) return 1
    const ln = /^\d+$/.test(l)
    const rn = /^\d+$/.test(r)
    if (ln && rn) {
      const diff = Number(l) - Number(r)
      if (diff !== 0) return diff
    } else if (ln !== rn) {
      // 数值标识小于字母标识
      return ln ? -1 : 1
    } else if (l !== r) {
      return l < r ? -1 : 1
    }
  }
  return 0
}

/** 解析版本字符串；格式不合法返回 null。 */
function parseVersion(input: string): { numeric: [number, number, number]; pre: string[] } | null {
  const text = input.trim().replace(/^v/i, '')
  if (!text) return null
  const [core, ...preParts] = text.split('-')
  const segments = core.split('.')
  if (segments.length > 3) return null
  const numeric: [number, number, number] = [0, 0, 0]
  for (let i = 0; i < 3; i++) {
    const seg = segments[i]
    if (seg === undefined) continue
    if (!/^\d+$/.test(seg)) return null
    numeric[i] = Number(seg)
  }
  const pre = preParts.length > 0 ? preParts.join('-').split('.') : []
  return { numeric, pre }
}
