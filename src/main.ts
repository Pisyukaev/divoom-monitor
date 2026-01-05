import { createApp } from 'vue';
import ElementPlus from 'element-plus';
import 'element-plus/dist/index.css';
import 'element-plus/theme-chalk/dark/css-vars.css';
import App from './App.vue';
import { useTheme } from './composables/useTheme';

const app = createApp(App);

app.use(ElementPlus);

// Инициализируем тему перед монтированием приложения
const { initTheme } = useTheme();
initTheme();

app.mount('#app');
