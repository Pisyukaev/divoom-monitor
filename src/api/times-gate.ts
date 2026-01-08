import { invoke } from '@tauri-apps/api/core';

import { commands } from '../constants';

export const invokeCommand = async (
  method: (typeof commands)[number],
  options: { ipAddress: string; value: string | number }
) => {
  try {
    const response = await invoke(method, options);

    return response;
  } catch (e) {
    console.error(`Error during update ${method} method: ${e}`);

    return null;
  }
};
