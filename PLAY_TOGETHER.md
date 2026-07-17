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

## Implementation (already in the repo)

| Piece | Where |
| --- | --- |
| Relay Durable Object (`LinkRoom`) | `server/src/link.ts` |
| Upgrade route `GET /v1/link/:code` + DO export | `server/src/index.ts` |
| DO binding + SQLite migration | `server/wrangler.toml` |
| Client tunnel (host/guest bridge) | `src-tauri/src/link.rs` |
| Tauri commands `link_host` / `link_join` / `link_stop` | registered in `src-tauri/src/lib.rs` |
| UI ("Host a session" / "Join with a code") | `src/lib/components/CactusLink.svelte` (in the Play Together panel) |

`LinkRoom` keeps the host's control socket and, for each guest, buffers bytes
until the host opens its paired data socket, then relays raw bytes between the
two. One WebSocket carries exactly one Minecraft TCP connection. The Rust client
(`link.rs`) bridges each WebSocket to a local TCP socket with
`tokio-tungstenite`.

## Deploying the relay

The relay ships inside the existing Worker in `server/`, so deploying is just:

```bash
cd server
wrangler deploy   # registers the LinkRoom DO class, its v1 migration, and the route
```

Notes:
- The DO binding (`LINK`) and the `[[migrations]]` block are already in
  `wrangler.toml`; the first `wrangler deploy` after this change creates the
  SQLite-backed class.
- No new secrets. The relay is unauthenticated per code — the code itself is the
  capability (like the share-code flow). To gate it, require a boards session
  token on the upgrade request in `server/src/index.ts`.
- The launcher derives `wss://…/v1/link/<code>` from the same
  `VITE_STREAMER_API_URL` base it already uses (`src/lib/boardApi.ts`), so no
  extra client config.

## How a session goes

1. Host opens the **Play Together** panel → *Host a session* → gets a code (their
   server must be running, default port 25565).
2. Host shares the code. Guests enter it under *Join* → the launcher returns a
   local port.
3. Guest opens Minecraft → Multiplayer → Direct Connect → `127.0.0.1:<port>`.

## Testing / caveats

This is real networking that can't be exercised from a single machine's unit
tests — verify with two machines on different networks:
- The host's server must actually be listening on the port passed to
  *Host a session*.
- rustls needs a crypto provider at runtime; if `connect_async` ever panics with
  "no process-level CryptoProvider", install one at startup
  (`rustls::crypto::ring::default_provider().install_default()`).
- Latency = round-trip through the relay. A later optimization is opportunistic
  P2P (hole-punch) with the relay as fallback.
```
