# LexiLog

A personal app for IELTS Academic practice: vocabulary cards scheduled with
FSRS, writing with drafts and rewrites, a practice calendar, a mistake log and
optional AI feedback. It runs on a home server and is accessed securely over
Tailscale using automatic HTTPS. Docker binds only to `127.0.0.1:1111`,
completely blocking unauthenticated access from the local Wi-Fi / LAN.

> [!WARNING]
> **Vibe-coded, for personal use only.** This project was built with an AI
> coding assistant for one person's own IELTS study. It has not been security
> reviewed or tested for production, and there is **no guarantee of security**.
>
> - There is no login, no user accounts and no rate limiting.
> - Access is secured exclusively via Tailscale mesh VPN (`tailscale serve`).
>   Never expose port 1111 to LAN (`0.0.0.0`) or port-forward it to the internet.
> - Do not use it as a product or run it for other people. If you want to,
>   treat this code as a starting point and do your own security review first.
>
> It is provided as is, without warranty (see [LICENSE](LICENSE)).

## Run with Docker (home server)

```bash
mkdir -p data backups          # host folders for the database and backups
cp .env.example .env           # optional: AI feedback settings, then add AI_API_KEY
docker compose up -d --build   # first build compiles Rust, takes a few minutes
tailscale serve --bg --yes --https=443 http://127.0.0.1:1111
```

Then open `https://<your-node>.<your-tailnet>.ts.net` (e.g. `https://alexander.taile859de.ts.net`).

The database is `./data/ielts.db` on the host. Rebuilding or upgrading the
image doesn't touch it. Migrations run automatically when the app starts.

## Backups

```bash
scripts/backup.sh              # writes backups/ielts-YYYY-MM-DD_HHMMSS.db, keeps newest 30
```

It uses `VACUUM INTO`, which is safe while the app is running. Nightly, via
`crontab -e`:

```
0 3 * * * /path/to/EnglishIELTSWebsite/scripts/backup.sh >> /path/to/EnglishIELTSWebsite/backups/backup.log 2>&1
```

To restore: `docker compose down`, copy a backup over `data/ielts.db`
(and delete any `data/ielts.db-wal` / `data/ielts.db-shm`), then `docker compose up -d`.

## AI feedback

The Writing editor can send a version to an AI model for estimated bands, a
list of issues with hints (as questions, never corrected text) and suggested
mistake tags. It uses any OpenAI-compatible `/chat/completions` gateway; the
defaults point at the university gateway.

- Settings live in `.env` (copy `.env.example`). Without `AI_API_KEY` the
  feature is off and everything else works.
- `AI_MODELS` lists the models and their daily token limits. The app counts
  its own usage per model (input + output) and resets at local midnight. The
  key may be used elsewhere too, so a `429` from the gateway is the final word;
  the editor then offers another model with quota left.
- One check costs roughly 2–3k tokens. Every call is stored in `ai_feedback`,
  failed ones too, so the usage count stays honest.
- The key stays on the server: the browser never sees it. `.env` is in
  `.gitignore`.

## Development

Two terminals:

```bash
cd backend && cargo run        # API on 127.0.0.1:1111, database in backend/data/, reads ../.env
cd frontend && npm run dev     # UI on 127.0.0.1:5173, forwards /api to :1111
```

Both listen on this machine only. To try the dev UI on a phone, use
`tailscale serve` for port 5173 rather than opening it to the LAN.

Checks: `cargo test`, `cargo clippy`, `npm run check`.

## Settings (environment variables)

| Variable            | Default           | Meaning                                  |
|---------------------|-------------------|------------------------------------------|
| `APP_BIND`          | `127.0.0.1:1111`  | Listen address (the Docker image sets `0.0.0.0:1111`) |
| `DATABASE_PATH`     | `data/ielts.db`   | SQLite file (folder is created)          |
| `STATIC_DIR`        | `../frontend/dist`| Built frontend; skipped if missing       |
| `APP_TZ`            | `Asia/Bangkok`    | Where "today" starts and ends            |
| `DESIRED_RETENTION` | `0.9`             | FSRS target recall (0.70 – 0.99)         |
| `AI_BASE_URL`       | university gateway| OpenAI-compatible API base URL           |
| `AI_API_KEY`        | (none)            | Gateway key; unset = AI feedback off     |
| `AI_MODELS`         | `claude-sonnet-5:200000` | `model:daily_tokens`, comma separated |
| `AI_DEFAULT_MODEL`  | first in the list | Model used unless another is picked      |

## Code layout

The backend follows clean architecture. Inner layers never import outer ones.

```
backend/src/
  domain/          data types and pure rules (word count, rating, calendar days,
                   heatmap levels, mistake trends, checking AI replies)
  application/     use cases (services) + ports = traits for storage, scheduling and AI
  infrastructure/  SQLite repositories (sqlx), the FSRS scheduler (fsrs crate),
                   the AI reviewer (reqwest, chat/completions)
  presentation/    HTTP handlers and routes (ntex)
  main.rs          builds the concrete parts and wires them together
backend/migrations/  SQL schema, applied in order at startup

frontend/src/
  app.css          "Ink & Signal" theme: colour/font variables and shared classes
  App.svelte       sidebar layout (top bar on phones), routes; Review runs full screen
  pages/           one component per screen
  lib/api/         typed calls to the backend
  lib/components/  Icon, CardForm, TagInput, Timer, VersionDiff, Heatmap,
                   PracticeForm, MistakeEditor, AiFeedbackPanel
```

## Scheduling rules

- FSRS-6 via the `fsrs` crate, default parameters. Every review is kept in
  `review_logs` so parameters can be optimised later.
- **Again** brings the card back in 10 minutes. Hard/Good/Easy are whole days.
- "Due today" means due before local midnight tonight (`APP_TZ`).

## Practice calendar and mistake log

- The calendar shows the last 53 weeks. A day's shade combines reviews,
  writing versions touched and logged practice minutes. There is no streak:
  a day off never resets anything.
- Mistakes are tagged on the piece (the first draft shows real habits). The
  Mistakes page shows each tag per 1000 words of first drafts, counting only
  pieces that were checked (tagged, or with feedback), and marks tags as
  fading, rising, steady or new.

## License

MIT, see [LICENSE](LICENSE).
