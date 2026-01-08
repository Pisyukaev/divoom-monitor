export const dateFormats = [
  {
    value: 'yyyy-mm-dd',
    label: 'yyyy-mm-dd',
  },
  {
    value: 'dd-mm-yyyy',
    label: 'dd-mm-yyyy',
  },
  {
    value: 'mm-dd-yyyy',
    label: 'mm-dd-yyyy',
  },
  {
    value: 'yyyy.mm.dd',
    label: 'yyyy.mm.dd',
  },
  {
    value: 'dd.mm.yyyy',
    label: 'dd.mm.yyyy',
  },
  {
    value: 'mm.dd.yyyy',
    label: 'mm.dd.yyyy',
  },
];

export const commands = [
  'scan_devices',
  'set_brightness',
  'set_switch_screen',
  'set_temperature_mode',
  'set_mirror_mode',
  'set_24_hours_mode',
  'get_device_info',
] as const;
