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

Docker (does not deploy anywhere by itself):

```bash
docker build -t letheans .
docker run --rm -p 3000:3000 --env-file .env.local letheans
```

Rust **1.99+** is required (`rust-toolchain.toml`). The binary reads `.env` then `.env.local` (later file wins), same order as Next.js.

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
- **CSS delivery.** Tailwind v4 is compiled ahead of time into `static/css/app.css` from `styles/input.css` (the original tokens and component CSS). Regenerating it needs Node: `npx @tailwindcss/cli -i styles/input.css -o static/css/app.css --minify`. The running server does not need Node.
- **404.** Unknown paths use the site header and footer and the sentence “This page could not be found.” Next’s default 404 is plainer and has no site header.
- **Static caching.** Files are served as-is, not as content-hashed immutable assets.
- **Whitelist card.** Drawing still happens in the browser on a 1600×900 canvas at 2× (3200×1800 PNG), with Download, Share on X, Copy image, and Copy caption. The copied state is the green `btn-success` style and the label “Copied ✓”.

Task verification defaults to the honor system in `src/verify.rs`: opening the task link (or pasting a valid X reply link and pressing Complete) is enough. Server-side X checks run only when `ENABLE_X_TASK_VERIFICATION=true`, a bearer token is set, and the quest has a `target_id`. API errors still fall open to `true`, same as the original.
