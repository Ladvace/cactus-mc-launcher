// Helpers for a server's connection address (derived from server.properties).

import { api } from "$lib/api";

export const DEFAULT_PORT = 25565;

/** Extract `server-port` from a server.properties body (defaults to 25565). */
export function parseServerPort(props: string): number {
  for (const line of props.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const match = trimmed.match(/^server-port\s*=\s*(\d+)/);
    if (match) return parseInt(match[1], 10) || DEFAULT_PORT;
  }
  return DEFAULT_PORT;
}

/** `host` alone when the port is default, else `host:port`. */
export function formatAddress(host: string, port: number): string {
  return port === DEFAULT_PORT ? host : `${host}:${port}`;
}

/** The `localhost[:port]` address for a server instance. */
export async function localServerAddress(id: string): Promise<string> {
  let port = DEFAULT_PORT;
  try {
    port = parseServerPort(await api.readServerProperties(id));
  } catch {
    /* no properties yet — assume the default port */
  }
  return formatAddress("localhost", port);
}
