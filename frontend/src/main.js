import { createApp } from 'vue'
import App from './App.vue'

// 基础令牌 + 通用小件 + 11 套主题
import './styles/base.css'
import './styles/components.css'
import './styles/themes.css'
// 应用外壳与业务视图（侧边栏 / 顶栏下拉 / 工作台 / 网络图 / 数据管理）
import './styles/app.css'

createApp(App).mount('#app')
