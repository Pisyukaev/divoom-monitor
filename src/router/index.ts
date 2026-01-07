import { createRouter, createWebHistory } from 'vue-router';
import DeviceList from '../components/device-list/list.vue';
import DeviceSettings from '../views/DeviceSettings.vue';

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'DeviceList',
      component: DeviceList,
    },
    {
      path: '/device/:id',
      name: 'DeviceSettings',
      component: DeviceSettings,
      props: true,
    },
  ],
});

export default router;

