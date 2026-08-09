// WebDAV 测试服务器 — 仅用于 zerolaunch-plugin-api 的 WebDAVStorageService 集成测试。
//
// 实现 WebDAV 协议的最小可用子集（PUT/GET/DELETE/PROPFIND/OPTIONS），
// 存储根目录为系统临时目录下 .zl-webdav-test-root，测试结束后残留由测试清理。
// 通过 `bun run webdav_server.ts` 启动，端口 18080。

import { mkdir, readFile, readdir, unlink, writeFile } from 'node:fs/promises'
import { join } from 'node:path'

const PORT = 18080
const ROOT = join(process.env.TEMP || '/tmp', '.zl-webdav-test-root')

await mkdir(ROOT, { recursive: true })

Bun.serve({
  port: PORT,
  async fetch(req) {
    const url = new URL(req.url)
    // 测试专用：POST /__shutdown 优雅退出（避免 bun run 子进程树残留）
    if (req.method === 'POST' && url.pathname === '/__shutdown') {
      setTimeout(() => process.exit(0), 50)
      return new Response('bye', { status: 200 })
    }
    const rel = decodeURIComponent(url.pathname).replace(/^\/+/, '')
    const target = join(ROOT, rel)

    switch (req.method) {
      case 'PUT': {
        const buf = Buffer.from(await req.arrayBuffer())
        await mkdir(join(target, '..'), { recursive: true })
        await writeFile(target, buf)
        return new Response(null, { status: 201 })
      }
      case 'GET': {
        try {
          return new Response(await readFile(target))
        } catch {
          return new Response('Not Found', { status: 404 })
        }
      }
      case 'DELETE': {
        try {
          await unlink(target)
          return new Response(null, { status: 204 })
        } catch {
          return new Response('Not Found', { status: 404 })
        }
      }
      case 'PROPFIND': {
        const entries = await readdir(target, { withFileTypes: true }).catch(() => [])
        const base = url.pathname.replace(/\/+$/, '')
        const xml = `<?xml version="1.0"?><d:multistatus xmlns:d="DAV:">${entries
          .map((e) => {
            const name = e.isDirectory() ? `${e.name}/` : e.name
            return `<d:response><d:href>${base}/${name}</d:href><d:propstat><d:prop><d:displayname>${e.name}</d:displayname><d:getlastmodified>Mon, 01 Jan 2024 00:00:00 GMT</d:getlastmodified></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>`
          })
          .join('')}</d:multistatus>`
        return new Response(xml, {
          status: 207,
          headers: { 'Content-Type': 'application/xml' },
        })
      }
      case 'OPTIONS':
        return new Response(null, { status: 200, headers: { DAV: '1' } })
      default:
        return new Response('Method Not Allowed', { status: 405 })
    }
  },
})

console.log(`WebDAV test server ready on ${PORT}`)
