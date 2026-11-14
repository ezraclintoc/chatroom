# do-chatroom

A realtime chat app written entirely in Rust, running on Cloudflare Workers.
Each chat room is a Durable Object with its own SQLite database, and the
frontend is a Leptos app compiled to WebAssembly.

## About this project

This is a **learning project**. I built it to learn Durable Objects,
WebSockets, and Leptos, none of which I had used before.

I wrote most of the Rust code by hand. I used AI as a guide: it helped me get
unstuck on errors, explained concepts as I went, and suggested which features
to build next. The HTML and CSS were the main parts written by AI.

## Features

* Rooms at `/room/:name`, created automatically the first time someone joins
* Live messages over WebSockets, using the Durable Object Hibernation API
* Message history (the last 50 messages are sent when you join)
* Typing indicators
* Messages from the same person grouped together
* Username remembered in the browser
* Server side validation and per connection rate limiting

## Stack

| Part      | Technology                                         |
| --------- | -------------------------------------------------- |
| Backend   | Cloudflare Workers, `workers-rs` (`worker` crate)  |
| State     | Durable Objects with SQLite storage                |
| Frontend  | Leptos 0.8 (CSR), `leptos_router`, `gloo-net`      |
| Build     | Trunk (frontend), `worker-build` via Wrangler      |
| Dev shell | Nix flake (fenix stable toolchain)                 |

## Project layout

```
do-chatroom/
├── flake.nix           # dev shell: Rust + wasm32 target, Node, Trunk, OpenSSL
├── Cargo.toml          # workspace root
├── shared/             # message types used by both sides (serde)
├── worker/             # Worker entry point and the Chatroom Durable Object
│   └── wrangler.toml
└── frontend/           # Leptos app
    ├── index.html
    ├── style.css
    └── src/
```

## How it works

1. The browser opens a WebSocket to `/api/room/:name/ws`.
2. The Worker looks up the Durable Object for that room name and forwards the
   request. The same name always reaches the same object.
3. The Durable Object accepts the socket, sends recent history, and stores
   new messages in its SQLite table before broadcasting them to everyone in
   the room.
4. Each socket carries a small attachment (rate limit bucket, last typing
   time) that survives hibernation.

### Protocol

All messages are JSON with a `type` field. Types live in `shared/`.

Client to server:

```json
{ "type": "send", "username": "ezra", "body": "hi" }
{ "type": "typing", "username": "ezra" }
```

Server to client:

```json
{ "type": "history", "messages": [ ... ] }
{ "type": "message", "id": 1, "username": "ezra", "body": "hi", "sent_at": 1794666442572 }
{ "type": "typing", "username": "ezra" }
{ "type": "error", "reason": "Slow down" }
```

`sent_at` is Unix time in milliseconds, set by the server.

### Limits

| Rule          | Value                                  |
| ------------- | -------------------------------------- |
| Username      | 1 to 32 characters, trimmed            |
| Message body  | 1 to 2000 characters, trimmed          |
| Messages      | Burst of 5, then 1 per second          |
| Typing events | At most 1 per second (extras dropped)  |
| History       | Last 50 messages per room              |

## Development

Enter the dev shell first:

```bash
nix develop        # or let direnv load it automatically
```

Then run the backend and frontend in two terminals:

```bash
# Terminal 1: backend
cd worker
npx wrangler dev

# Terminal 2: frontend, rebuilds into frontend/dist on save
cd frontend
trunk watch
```

Open http://localhost:8787. Wrangler serves both the frontend files and the
API, the same way production does. Refresh the page after changes.

**Alternative with live reload:** run `trunk serve` instead of `trunk watch`
and open http://localhost:8080. The frontend detects port 8080 and connects to
Wrangler on 8787 directly. (Trunk's WebSocket proxy is not used, because it
drops frames when the browser negotiates `permessage-deflate` compression.)

### Testing the API with websocat

```bash
websocat ws://localhost:8787/api/room/test/ws
{"type":"send","username":"ezra","body":"hello"}
```

### Resetting local data

Stop `wrangler dev` and delete `worker/.wrangler/state`. Every room starts
empty again. Do this after changing the table schema.

## Deploying

```bash
cd frontend
trunk build --release

cd ../worker
npx wrangler deploy
```

Wrangler uploads the Worker and everything in `frontend/dist` as static
assets. Requests to `/api/*` run the Worker; everything else is served from
the assets, with unknown paths falling back to `index.html` so room links work
on refresh.

Runs on the Workers Free plan, which supports SQLite backed Durable Objects.

## NixOS notes

* `pkg-config`, `openssl`, and `LD_LIBRARY_PATH` are set in the flake so
  `worker-build` can compile and run.
* The `wasm32-unknown-unknown` target is added through fenix in `flake.nix`,
  not `rust-toolchain.toml`.
* Flakes only see files tracked by git, so `git add` new files before
  running `nix develop`.

## Ideas for later

* Presence (who's online, join and leave notices)
* Connection status and automatic reconnect
* Loading older messages
* Date dividers and clickable links