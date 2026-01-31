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

export const TEXT_ALIGNMENT_OPTIONS: { label: string; value: number }[] = [
  { label: 'Scroll', value: 0 },
  { label: 'Normal', value: 1 },
  { label: 'Middle', value: 2 },
  { label: 'Right', value: 3 },
  { label: 'Left', value: 4 },
];

export const FONT_OPTIONS: { label: string; value: number }[] = [
  { label: 'Arial', value: 0 },
  { label: 'Times New Roman', value: 1 },
  { label: 'Verdana', value: 2 },
  { label: 'Courier New', value: 3 },
  { label: 'Georgia', value: 4 },
  { label: 'Garamond', value: 5 },
  { label: 'Comic Sans MS', value: 6 },
  { label: 'Impact', value: 7 },
];

export const commands = [
  'scan_devices',
  'set_brightness',
  'set_switch_screen',
  'set_temperature_mode',
  'set_mirror_mode',
  'set_24_hours_mode',
  'get_device_info',
  'upload_image_from_url',
  'upload_image_from_file',
  'set_screen_text',
  'reboot_device',
] as const;
