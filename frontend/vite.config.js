import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'

/**
 * 后端地址（反代目标）。
 *
 * 默认本机 8787。根目录的「启动Web服务.ps1」会在后端端口被占用、自动改口之后
 * 用环境变量 BCRM_API_TARGET 把它传给这里 —— 这样**端口变了也不用改一行前端代码**，
 * 更不用重新构建产物。
 */
const apiTarget = process.env.BCRM_API_TARGET || 'http://127.0.0.1:8787'

/**
 * dev / preview 都把 `/api` 反代到后端。
 *
 * 为什么需要：浏览器里前端跑在 5273、后端在 8787，属于跨源请求。虽然有 CORS 兜底，
 * 但走同源反代更干净 —— 请求不再是跨源，`file://`、代理、公司网络策略都不会来捣乱。
 * 配合 `client.js` 里的相对路径 `/api/v1`，整套前端与「后端到底在哪个端口」彻底解耦。
 */
const proxy = { '/api': { target: apiTarget, changeOrigin: true } }

export default defineConfig({
  base: './',
  plugins: [vue()],
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  server: { port: 5273, strictPort: false, proxy },
  preview: { proxy },
  build: {
    outDir: 'dist',
    assetsDir: 'assets',
    cssCodeSplit: false,
    chunkSizeWarningLimit: 1200,
  },
})
