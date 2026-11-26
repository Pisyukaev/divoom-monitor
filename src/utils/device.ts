const IPV4 = /^(\d{1,3}\.){3}\d{1,3}$/;

/** Device routes use the (URL-encoded) IP as id; returns the IP or null if the id is not an IP. */
export function parseDeviceIp(routeId: string | undefined): string | null {
  if (!routeId) return null;
  const decoded = decodeURIComponent(routeId);
  return IPV4.test(decoded) ? decoded : null;
}
