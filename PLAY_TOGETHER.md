# Play Together — share your server over the internet

Let a friend join a self-hosted world from a different network **without**
port-forwarding, a static IP, or Hamachi. The host brings their own free
[ngrok](https://ngrok.com) authtoken; everything else is built into the launcher.

## How it works

The launcher embeds the ngrok Rust SDK (no separate program to download). When
the host starts sharing, we open a TCP edge on ngrok that forwards to the local
Minecraft server and hand back a public `host:port`. Friends just use Minecraft's
**Direct Connect** with that address — nothing to install on their side.

```
Host machine                         ngrok edge                 Friend
┌────────────────────┐   embedded    ┌───────────┐            ┌──────────────┐
│ MC server :25565   │◀──ngrok SDK──▶│ 0.tcp.ngrok│◀──internet─│ Direct Connect│
│ launcher forwards  │   (outbound)  │ .io:12345 │            │ 0.tcp…:12345 │
└────────────────────┘               └───────────┘            └──────────────┘
```

Only the host needs an account, and only outbound connections are made, so home
routers/NAT are never in the way. Cost lives on the host's own ngrok free tier,
not on us.

## Using it (host)

1. Make a free ngrok account and copy your authtoken from
   <https://dashboard.ngrok.com/get-started/your-authtoken>.
2. Add the token once in **Settings → Servers → ngrok authtoken** (a single
   server can override it with its own key from its **Share online** row).
3. Open your **server instance** and start it (default port 25565).
4. Next to the server address, hit **Share via ngrok**. Share the shown
   `host:port` with friends — they open Minecraft → Multiplayer → Direct Connect
   → paste it.

The tunnel lives with the server (not the "Play Together" matchmaking panel):
the key is resolved as **per-server token → global Settings token**, and the
port is read from the instance's `server.properties`.

## Implementation

| Piece | Where |
| --- | --- |
| Tunnel (ngrok SDK, forward loop) | `src-tauri/src/tunnel.rs` |
| Commands `tunnel_start` / `tunnel_stop` | registered in `src-tauri/src/lib.rs` |
| API wrappers | `src/lib/api.ts` (`tunnelStart` / `tunnelStop`) |
| Global key | `Settings` (`ngrokAuthtoken`); per-server override on the instance |
| UI | `src/lib/components/ServerShare.svelte` (on the server instance page) |

No backend/deploy is required — the tunnel is entirely client-side plus the
host's ngrok account. `tunnel_start(authtoken, port)` connects an ngrok session,
opens a TCP endpoint, forwards each incoming connection to `127.0.0.1:<port>`,
and returns the public address; `tunnel_stop` tears it down.

## Caveats / testing

- Real networking — verify with two machines on different networks. The server
  must actually be listening on the port read from `server.properties`.
- ngrok's free tier has session/bandwidth limits; heavy servers may want a paid
  plan. Minecraft traffic is light, so casual play is fine.
- rustls needs a crypto provider at runtime; if `connect()` ever panics with
  "no process-level CryptoProvider", install one at startup
  (`rustls::crypto::ring::default_provider().install_default()`).
