import { createRouter, createWebHistory } from 'vue-router';
import DeviceList from '../components/device-list/list.vue';
import DeviceSettingsView from '../views/DeviceSettingsView.vue';
import CommonSettingsView from '../views/CommonSettingsView.vue';
import DisplaySettingsView from '../views/DisplaySettingsView.vue';
import SystemMetricsView from '../views/SystemMetricsView.vue';

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
      component: DeviceSettingsView,
      props: true,
      children: [
        {
          path: '',
          redirect: (to) => {
            return { path: `/device/${to.params.id}/common` };
          },
        },
        {
          path: 'common',
          name: 'CommonSettings',
          component: CommonSettingsView,
          props: true,
        },
        {
          path: 'display',
          name: 'DisplaySettings',
          component: DisplaySettingsView,
          props: true,
        },
      ],
    },
    {
      path: '/system',
      name: 'SystemMetrics',
      component: SystemMetricsView,
    },
  ],
});

router.onError((error) => {
  console.error('Router error:', error);
});

export default router;
