import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { copyFileSync, mkdirSync, existsSync, readdirSync } from "fs";
import { join, resolve } from "path";

const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
// 自定义插件：复制i18n locales到src-tauri
const copyI18nPlugin = () => {
  return {
    name: 'copy-i18n-locales',
    buildStart() {
      const srcLocalesDir = join(process.cwd(), 'src-ui', 'i18n', 'locales');
      const destDir = join(process.cwd(), 'src-tauri', 'locales');

      if (!existsSync(srcLocalesDir)) return;

      if (!existsSync(destDir)) {
        mkdirSync(destDir, { recursive: true });
      }
      try {
        const files = readdirSync(srcLocalesDir);
        files.forEach(file => {
          const srcFile = join(srcLocalesDir, file);
          const destFile = join(destDir, file);
          copyFileSync(srcFile, destFile);
        });
        console.log(`${files.length} i18n locales files copied to src-tauri/locales/`);
      } catch (error) {
        console.error('Failed to copy i18n locales:', error);
      }
    }
  };
};

export default defineConfig(async () => ({
  plugins: [vue(), copyI18nPlugin()],

  resolve: {
    alias: {
      '@': resolve(__dirname, 'src-ui'),
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 12345,
    strictPort: true,
    host: host || false,
    // 显式关闭 forwardConsole：其默认值由 determineAgent()（检测 AI agent 环境变量如
    // CLAUDECODE/AI_AGENT）决定，会导致 vite:forward-console 插件在不同启动上下文
    // （agent 终端 vs 普通终端）下有无不同，进而改变 optimizeDeps 的 configHash、
    // 使依赖预构建缓存互相失效、每次切换上下文都触发 60-130s 全量重优化。
    forwardConsole: false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`
      // 本仓库为 Cargo workspace，target/（构建产物，当前约 116GB/9.4 万文件）位于仓库根目录，
      // 不在 src-tauri/ 下；若不排除，chokidar 会递归监听它，启动后 CPU 100%、内存 1GB+、
      // 事件循环被占死，页面请求排队 60-80s。
      ignored: ["**/src-tauri/**", "**/target/**"],
    },
  },

  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        setting_window: resolve(__dirname, 'setting_window.html'),
      },
    },
  },
}));
