import { api } from "$lib/api";

export const DEFAULT_PORT = 25565;

export function parseServerPort(props: string): number {
  for (const line of props.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const match = trimmed.match(/^server-port\s*=\s*(\d+)/);
    if (match) return parseInt(match[1], 10) || DEFAULT_PORT;
  }
  return DEFAULT_PORT;
}

export function formatAddress(host: string, port: number): string {
  return port === DEFAULT_PORT ? host : `${host}:${port}`;
}

export async function localServerAddress(id: string): Promise<string> {
  let port = DEFAULT_PORT;
  try {
    port = parseServerPort(await api.readServerProperties(id));
  } catch {
  }
  return formatAddress("localhost", port);
}
