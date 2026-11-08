# Letheans — Rust port

The Letheans NFT site (home, whitelist apply, status lookup) rebuilt in Rust so the pages, copy, routes, and API behavior match the Next.js app.

Stack: **Axum**, **Askama** templates, **vanilla JS**, and a precompiled **Tailwind CSS v4** stylesheet that uses the original design tokens. Google Sheets and X (Twitter) OAuth 2.0 use the same environment variable names as the Next.js app.

## Run

```bash
cp .env.example .env.local   # optional; empty env is demo mode
cargo run                    # http://localhost:3000
```

Release build:

```bash
cargo build --release
PORT=3000 ./target/release/letheans
```

Checks:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo run -- --sheet-check   # prints OK or an error; needs the Google env vars
```

Without `X_CLIENT_ID`, `X_CLIENT_SECRET`, `AUTH_SECRET`, and the three Google Sheets variables, the site runs in **demo mode**: pages load, `/apply` shows the demo banner, `POST /api/apply` and `GET /api/wl-status` return **503**. `/api/stats` returns `{"applicants":null,"demo":true}`.

Docker (local image only; this is not the Vercel deploy path):

```bash
docker build -t letheans .
docker run --rm -p 3000:3000 --env-file .env.local letheans
```

Rust **1.99.1** is pinned in `rust-toolchain.toml`. The binary reads `.env` then `.env.local` (later file wins), same order as Next.js.

## Deploy on Vercel

This repo uses Vercel's **Rust runtime** (Beta, all plans, Fluid Compute): one Axum function at `api/app.rs`, built with [`vercel_runtime`](https://vercel.com/docs/functions/runtimes/rust) 2.4. `vercel.json` rewrites every path to `/api/app`, and the runtime passes the original URL through, which is the same pattern as [Vercel's Axum example](https://github.com/vercel/examples/tree/main/rust/axum). `cargo run` is unchanged: it still starts `src/main.rs` on `0.0.0.0:$PORT`.

Static files live in `public/`. That directory is the deployment output, so Vercel serves `/logo.png`, `/css/app.css`, `/js/*`, `/fonts/*`, and `/nft/*` from the CDN before the rewrite. The function also embeds those files and serves them if the disk path is not mounted.

### Import settings

In the Vercel dashboard: **Add New → Project**, import `Skyzee2704/rust-web`, then set these before the first deploy. `vercel.json` already forces the same framework, output directory, and Fluid Compute, so leave the overrides off unless a value below is blank.

| Project setting | Value |
| --- | --- |
| Framework Preset | **Other** |
| Root Directory | **`./`** (repository root; do not pick a subdirectory) |
| Build Command | **empty, Override off** (do not set `npm run build`; the Rust runtime runs `cargo build --release`) |
| Output Directory | **`public`** |
| Install Command | **empty, Override off** (there is no Node install) |
| Node.js Version | leave the default; the site does not run on Node |

Production and Preview both need the env vars below. Do not commit values.

### Env vars to add

Set these under **Settings → Environment Variables** for Production (and Preview if you want preview deploys to leave demo mode). Names match `.env.example`.

| Name | Production value |
| --- | --- |
| `NEXT_PUBLIC_SITE_URL` | `https://<your-domain>` with no trailing slash. Use the production `*.vercel.app` host until a custom domain is attached. |
| `AUTH_URL` | The same URL as `NEXT_PUBLIC_SITE_URL`. |
| `AUTH_SECRET` | `openssl rand -base64 32` |
| `X_CLIENT_ID` | X OAuth 2.0 client ID |
| `X_CLIENT_SECRET` | X OAuth 2.0 client secret |
| `GOOGLE_SERVICE_ACCOUNT_EMAIL` | service account `client_email` |
| `GOOGLE_PRIVATE_KEY` | service account PEM. Paste the key with literal `\n` between lines, no surrounding quotes in the Vercel UI. |
| `GOOGLE_SHEET_ID` | spreadsheet id. Share the sheet with the service account as **Editor**. |
| `IP_HASH_SALT` | a long random string |
| `GOOGLE_SHEET_TAB` | optional, default `Applications` |
| `ENABLE_X_TASK_VERIFICATION` | optional, leave unset or `false` |
| `X_BEARER_TOKEN` | optional, only if verification is `true` |
| `UPSTASH_REDIS_REST_URL` / `UPSTASH_REDIS_REST_TOKEN` | optional, but recommended on Vercel (see limits) |

`PORT` is not used on Vercel. The Rust runtime listens on its own loopback port.

### X OAuth callback

In the X app (User authentication settings, OAuth 2.0, Web App), set the callback URL to:

```text
https://<your-domain>/api/auth/callback/twitter
```

`<your-domain>` is the host in `AUTH_URL` (for example `letheans.vercel.app` or `letheans.xyz`). Also set the Website URL to `https://<your-domain>`. Add a second callback only if a preview deployment should log in on its own host; each preview host is different, and `AUTH_URL` is a single value, so production login is the one to register first.

Scopes stay `users.read tweet.read offline.access`.

### Limits

- The Rust runtime is **Beta**. New projects run on Fluid Compute, which reuses a warm instance, but a cold start still loads the native binary before the first request.
- This function's `maxDuration` is **60 seconds** (`vercel.json`). The platform default is 300s on every plan, including Hobby; Pro can go higher. Sheets, X, and the avatar proxy are short calls (the avatar client times out at 6s).
- Request and response bodies are capped at **4.5 MB** by Vercel. Apply payloads are tiny. The avatar proxy refuses images over 2 MB before that.
- The standard function bundle limit is **250 MB**. The release binary plus embedded `public/` assets are far under that.
- **In-memory rate limits, the applicant-count cache, the GTD/FCFS tab cache, and the Google access token are per instance.** Serverless (and Fluid) runs more than one instance, so the 5/min and 20/min windows are not global. Set the Upstash variables for a shared limiter. A warm instance still applies the in-memory window to requests it handles itself.
- Env values have to fit Vercel's variable size. A normal service-account PEM does. Do not put the raw JSON key file in the repo.

## Env

Same names as the original `.env.example`. `PORT` is the only extra (default `3000`).

| Variable | Required | Notes |
| --- | --- | --- |
| `NEXT_PUBLIC_SITE_URL` | recommended | Public URL for Open Graph. Default `http://localhost:3000`. |
| `AUTH_SECRET` | yes, for live login | `openssl rand -base64 32` |
| `AUTH_URL` | yes, in production | Callback is `{AUTH_URL}/api/auth/callback/twitter` |
| `X_CLIENT_ID` / `X_CLIENT_SECRET` | yes | X OAuth 2.0 web app. Scopes: `users.read tweet.read offline.access` |
| `GOOGLE_SERVICE_ACCOUNT_EMAIL` | yes | Service account `client_email` |
| `GOOGLE_PRIVATE_KEY` | yes | PEM; literal `\n` is turned into newlines |
| `GOOGLE_SHEET_ID` | yes | Spreadsheet id. Share the sheet with the service account as Editor |
| `GOOGLE_SHEET_TAB` | no | Default `Applications`. Header row is created if the tab is empty |
| `IP_HASH_SALT` | yes | Salt for IP and user-agent hashes. Raw IP is not stored |
| `ENABLE_X_TASK_VERIFICATION` | no | `true` plus `X_BEARER_TOKEN` turns on best-effort X API checks. Default is the honor system |
| `X_BEARER_TOKEN` | no | Only used when verification is on |
| `UPSTASH_REDIS_REST_URL` / `UPSTASH_REDIS_REST_TOKEN` | no | Shared rate limit. In-memory windows are used when these are unset or Redis errors |
| `PORT` | no | Listen port, default `3000` |

Sheet columns, in order: `timestamp, x_user_id, x_username, wallet, tasks_completed, reply_url, ua_hash, ip_hash`.

GTD and FCFS are separate tabs of wallet addresses (column A). GTD wins if a wallet is on both. A wallet on either tab counts as whitelisted even if it is not in Applications.

## Routes

- `/` — hero, marquee, lore, legends deck, unrevealed count, how to join, mint countdown and ticket, FAQ, footer
- `/apply` — connect X, quests, EVM wallet, duplicate block, registration card
- `/status` — lookup by wallet or X username
- `POST /api/apply` — same-origin check, 5/min rate limit, honeypot, checksummed wallet, honor-system tasks, Sheets append. Registration number is the sheet row minus the header
- `GET /api/wl-status` — `?q=` wallet or X username, `?me=1` for the signed-in account. Masked wallet, 20/min, `Cache-Control: no-store`, GTD/FCFS cached ~10s
- `GET /api/avatar?u=` — same-origin proxy of `unavatar.io` so the card canvas is not tainted
- `GET /api/stats` — applicant count, cached 60s
- `/api/auth/signin/twitter`, `/api/auth/callback/twitter`, `/api/auth/signout`, `/api/auth/session`

## What is not identical

- **Runtime.** The UI is server-rendered HTML plus a small script, not React 19 or Next.js. There is no hydration pass. The signed-in apply form is known on the first response, so there is no auth “loading” state.
- **Motion.** Framer Motion is approximated with CSS (letter blur-in, scroll reveals, word reveals, marquee, petals, bob, blink) and a small pointer spring for the cursor. The legend deck still flicks at the original thresholds (80px or 500px/s). Springs are not a physics match for every Framer transition.
- **Session cookie.** X OAuth 2.0 PKCE uses the same callback path and scopes, and the session lasts 7 days, but the cookie is an HMAC token named `letheans_session`, not an Auth.js JWT. There is no `/api/auth/csrf` endpoint. OAuth failures return to `/apply?error=…` with a short message instead of the Auth.js error page. Disconnect is a GET so the control can live inside the application form.
- **Upstash.** The optional Redis limiter is a sliding window stored in a sorted set (prefixes `letheans:apply` and `letheans:status`, same 5/min and 20/min caps). It is not the `@upstash/ratelimit` Lua script. The in-memory limiter is the original fixed window and is what tests cover.
- **Fonts and images.** Playfair Display, Inter, and IBM Plex Mono (latin) are self-hosted. `next/font` size-adjusted fallbacks and `next/image` are not used. Italic on the page is synthesized, which matches the original font setup (no italic file was requested there).
- **CSS delivery.** Tailwind v4 is compiled ahead of time into `public/css/app.css` from `styles/input.css` (the original tokens and component CSS). Regenerating it needs Node: `npx @tailwindcss/cli -i styles/input.css -o public/css/app.css --minify`. The running server does not need Node.
- **404.** Unknown paths use the site header and footer and the sentence “This page could not be found.” Next’s default 404 is plainer and has no site header.
- **Static caching.** Files are served as-is, not as content-hashed immutable assets.
- **Whitelist card.** Drawing still happens in the browser on a 1600×900 canvas at 2× (3200×1800 PNG), with Download, Share on X, Copy image, and Copy caption. The copied state is the green `btn-success` style and the label “Copied ✓”.

Task verification defaults to the honor system in `src/verify.rs`: opening the task link (or pasting a valid X reply link and pressing Complete) is enough. Server-side X checks run only when `ENABLE_X_TASK_VERIFICATION=true`, a bearer token is set, and the quest has a `target_id`. API errors still fall open to `true`, same as the original.
