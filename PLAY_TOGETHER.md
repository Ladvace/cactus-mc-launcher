# Play Together — zero-install server linking ("Cactus Link")

Goal: let a player host a world for friends **without** port-forwarding, a static
IP, or third-party software like Hamachi/ZeroTier. Everything the guest needs
ships inside the launcher.

## Why this works without any download

Hamachi/ZeroTier install a virtual network adapter (TUN/TAP driver) and route a
fake LAN over their relays — hence the install + admin rights. We don't need a
virtual LAN. We only need to get one TCP stream (Minecraft Java is plain TCP on
`:25565`) from a guest to the host.

The trick: **both peers make only _outbound_ connections** to a relay we run, so
home routers and firewalls never need a port opened. The relay stitches the two
streams together.

```
Host machine                     Cloudflare (relay)                Guest machine
┌───────────────────┐            ┌──────────────────┐            ┌───────────────────┐
│ MC server :25565  │            │  LinkRoom (DO)   │            │ MC client         │
│      ▲            │  WebSocket │  code ABC123     │  WebSocket │   │ connects to    │
│      │  TCP       │◀──────────▶│  pairs host⇄guest│◀──────────▶│   ▼ 127.0.0.1:PORT│
│ launcher bridge   │  (outbound)│  pipes bytes     │  (outbound)│ launcher bridge   │
└───────────────────┘            └──────────────────┘            └───────────────────┘
```

- **Host launcher:** runs the MC server on `localhost:25565`, opens a WebSocket to
  `wss://<api>/v1/link/<code>?role=host`. For each guest stream, it opens a fresh
  TCP connection to `127.0.0.1:25565` and bridges bytes ⇄ WebSocket.
- **Guest launcher:** opens a local TCP listener on `127.0.0.1:<port>` and opens a
  WebSocket to the same code with `role=guest`. Minecraft connects to that local
  port; the launcher bridges the TCP ⇄ WebSocket. The guest just sees a normal
  "Direct Connect" to `localhost:<port>`.
- **LinkRoom Durable Object:** one instance per code. Holds the host + guest
  sockets and relays frames between them. One WebSocket = one TCP stream, so
  multiple guests (and Minecraft's separate status-ping vs login connections)
  each get their own paired sockets — no multiplexing protocol needed.

This is the same idea behind e4mc / playit.gg / Essential's "invite friends",
but self-hosted and branded, with no external dependency for the user.

## Alternatives (and why the relay is the MVP)

| Option | Pros | Cons |
| --- | --- | --- |
| **DO relay (this doc)** | Zero install, works behind any NAT, fits our stack | Traffic flows through our relay (cheap, but not free at scale) |
| Direct P2P (ICE/STUN hole-punch) | Low latency, ~no relay cost | TCP hole-punching is unreliable; needs a UDP tunnel + TURN fallback anyway. Good **later** as an upgrade — try P2P, fall back to relay. |
| Embed ZeroTier/Tailscale (libzt/tsnet) | Full virtual LAN, battle-tested | Large native dep, bigger binary, still needs coordination/relay infra |
| Third-party (playit.gg, ngrok) | No build work | That *is* an external service; branding/free-tier limits |

Start with the relay; add opportunistic P2P as a v2 optimization.

## Cost note

Minecraft traffic is light (~5–20 KB/s per player, with short spikes on world
join). Cloudflare bills Durable Objects on requests + active duration; WebSocket
**hibernation** keeps idle rooms cheap. SQLite-backed Durable Objects are
available on the **free** Workers plan, and egress from Workers is not metered —
but verify current Cloudflare pricing before relying on it. If relay bandwidth
ever dominates, a tiny always-on VPS running a dumb TCP relay is a drop-in
alternative to the DO.

---

## Deploying the relay

The relay is a Durable Object added to the existing Worker in `server/`.

### 1. Add the Durable Object + route

Create `server/src/link.ts` — the room that pairs the two sockets:

```ts
// One instance per session code. Pairs a host socket with guest sockets and
// relays raw bytes between them. Each WebSocket carries exactly one TCP stream.
export class LinkRoom {
  private host?: WebSocket;
  private waitingGuests: WebSocket[] = [];

  constructor(private state: DurableObjectState) {}

  async fetch(req: Request): Promise<Response> {
    const role = new URL(req.url).searchParams.get("role");
    const pair = new WebSocketPair();
    const [client, server] = [pair[0], pair[1]];
    server.accept();

    if (role === "host") {
      this.host = server;
      server.addEventListener("close", () => (this.host = undefined));
    } else {
      // A guest stream: pair it with a fresh host-side stream. In practice the
      // host opens its own paired WS on demand; here we relay 1:1 once both ends
      // are present. Buffer until the host stream for this guest arrives.
      this.waitingGuests.push(server);
      // ...relay logic: forward server messages to the paired host socket and
      // vice-versa; close both when either closes.
    }
    return new Response(null, { status: 101, webSocket: client });
  }
}
```

> The full pairing/relay implementation (matching a guest WS to a host WS per
> stream, back-pressure, and close propagation) lives in `server/src/link.ts`
> when the client side lands. The sketch above shows the shape.

Mount an upgrade route in `server/src/index.ts`:

```ts
app.get("/v1/link/:code", (c) => {
  if (c.req.header("Upgrade") !== "websocket")
    return c.text("expected websocket", 426);
  const id = c.env.LINK.idFromName(c.req.param("code"));
  return c.env.LINK.get(id).fetch(c.req.raw);
});
```

Export the class from the Worker entry (`server/src/index.ts`):

```ts
export { LinkRoom } from "./link";
```

### 2. Bind the Durable Object in `server/wrangler.toml`

```toml
[[durable_objects.bindings]]
name = "LINK"
class_name = "LinkRoom"

# SQLite-backed DO (free-plan eligible). New-class migration:
[[migrations]]
tag = "v1"
new_sqlite_classes = ["LinkRoom"]
```

Add `LINK: DurableObjectNamespace` to the `Env` interface in `server/src/types.ts`.

### 3. Deploy

```bash
cd server
wrangler deploy          # registers the DO class + migration and the route
```

No new secrets are required — the relay is unauthenticated per code, and the
code itself is the capability (like the share-code flow). If you want to gate it,
require a boards session token on the upgrade request.

### 4. Point the launcher at it

The launcher already reads the API base from `VITE_STREAMER_API_URL`
(see `src/lib/boardApi.ts`). The tunnel client derives the `wss://…/v1/link/<code>`
URL from that same base — no extra config.

---

## Client side (in the launcher — no user download)

Implemented in the Tauri Rust backend so it ships with the app:

- **`link::host(code)`** — connect the control WebSocket; for each new guest
  stream, `TcpStream::connect(("127.0.0.1", 25565))` and copy bytes both ways
  (`tokio::io::copy_bidirectional`) between the TCP socket and the WS.
- **`link::join(code) -> u16`** — bind `TcpListener` on `127.0.0.1:0`, return the
  port; on each inbound TCP connection open a guest WS and bridge it. The UI then
  tells the player to Direct Connect to `127.0.0.1:<port>` (or auto-fills it).
- Surface both in the existing **Play Together** panel: "Host a session" (shows
  the code) and "Join with a code".

Tauri commands to add: `link_host(code)`, `link_join(code) -> port`, `link_stop()`.
WebSocket in Rust via `tokio-tungstenite` (already compatible with the tokio
runtime Tauri uses).
```
