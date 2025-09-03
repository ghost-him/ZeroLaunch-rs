import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { copyFileSync, mkdirSync, existsSync, readdirSync } from "fs";
import { join } from "path";

const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
// 自定义插件：复制i18n locales到src-tauri
const copyI18nPlugin = () => {
  return {
    name: 'copy-i18n-locales',
    buildStart() {
      const srcLocalesDir = join(process.cwd(), 'src', 'i18n', 'locales');
      const destDir = join(process.cwd(), 'src-tauri', 'i18n');
      const destLocalesDir = join(destDir, 'locales');
      
      // 创建目标目录
      if (!existsSync(destDir)) {
        mkdirSync(destDir, { recursive: true });
      }
      if (!existsSync(destLocalesDir)) {
        mkdirSync(destLocalesDir, { recursive: true });
      }
      // 复制locales文件夹中的所有文件
      try {
        const files = readdirSync(srcLocalesDir);
        files.forEach(file => {
          const srcFile = join(srcLocalesDir, file);
          const destFile = join(destLocalesDir, file);
          copyFileSync(srcFile, destFile);
        });
        console.log(`✓ ${files.length} i18n locales files copied to src-tauri/i18n/locales/`);
      } catch (error) {
        console.error('Failed to copy i18n locales:', error);
      }
    }
  };
};

export default defineConfig(async () => ({
  plugins: [vue(), copyI18nPlugin()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 12345,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
