# Security

## What dbt-edith assumes

dbt-edith is a local tool. One person runs it on their own machine, against a
project they already have full access to, and it runs with that person's
rights: the terminal is their shell, the git buttons use their credentials,
the editor reads and writes their files.

- **The server listens on `127.0.0.1` only.** Other machines cannot reach it.
- **The browser is not trusted.** Any web page can talk to localhost, so every
  request must carry a `Host` naming this server and, for anything that is not
  a plain read, an `Origin` that is this server's own page and a
  `Sec-Fetch-Site` that does not say otherwise. Everything else is refused with
  `403`. The WebSocket terminal requires the `Origin`.
- **Headers are not proof, so the API wants a key.** Any program on the
  machine, under any account, can write the right `Host` and `Origin`. Each
  launch draws a 256-bit key and prints it in the link it opens; opening that
  link swaps it for an `HttpOnly`, `SameSite=Strict` cookie. Every `/api/` and
  `/ws/` request needs that cookie or an `Authorization: Bearer` header with
  the key, reads included, and gets `401` without it (0052).
- **Every reply carries a content security policy** that allows nothing from
  another origin, lets the terminal's WebSocket reach this port alone, and
  refuses framing, so the terminal cannot be put under an invisible overlay on
  someone else's page. Replies are also `no-store`, so no file, `.env` or
  profile lands in the browser's disk cache, and carry
  `Cross-Origin-Resource-Policy` and `Cross-Origin-Opener-Policy` set to
  `same-origin`.
- **Every file path is confined to the opened project.** `..`, absolute paths,
  drive letters and symlinks pointing out are refused. One file outside it is
  reachable: the dbt profile, found where dbt looks for it, and served only by
  its own route. No path for it ever comes from the browser (0017, 0049).
- **The profile's secrets stay on the server.** Its route replaces every
  password, token and private key with a placeholder before sending it, and
  puts the value on disk back when the placeholder is saved unchanged (0054).
  A save never widens the file's permissions, and the temporary file it goes
  through is readable by its owner alone from its first byte.
- **Opening a project runs nothing in it.** The venv in the status bar is read
  from its files, never asked by running its `python` or `dbt`, and a venv git
  tracks, which came with the repository, is never run at all (0053).
- **The editor reads the whole project, `.env` included.** That is what an
  editor is for. The environments panel, by contrast, never returns a `.env`
  value, only names and counts. The hover card on a variable does show a
  resolved value, but never one whose name is a `DBT_ENV_SECRET_*` or reads as a
  credential; both guards are server-side (0019). Searching across file contents
  skips `.env` files entirely, since a search is a wide read nobody aimed at a
  particular file (0020). A `profiles.yml` kept inside the project is one of its
  files: opened from the tree, it shows as it is, like a `.env`.
- **No outbound network calls of its own**, apart from the git commands you
  click. Snowflake column lineage is a separate script, `tools/sf_lineage.py`,
  which dbt-edith starts only while Snowflake is the column lineage tool you
  picked, and which opens a connection only when you click a column. It reads your dbt profile itself, so
  no credential passes through dbt-edith.

## Out of scope

- The project you open is yours. Its git hooks run when you commit, as they
  would from the command line; a clone brings none. With Snowflake picked as
  the column lineage tool, a virtual environment of the project that git does
  not track runs the lineage script.
- Anything already running as your user on the same machine. It can read the
  printed key from the terminal, or the cookie from the browser's profile.
- Another account reading the key off the browser's command line, on a Unix
  machine it shares with you, when dbt-edith opens the browser itself. Start it
  with `--no-open` and paste the link there.
- Exposing the port to the network with a tunnel or a proxy. The key travels
  in plain HTTP, which is private on the loopback and nowhere else, and the
  server was never meant to be reached that way.

## Known issues

- **CodeMirror 5.65.21 carries CVE-2025-6493**, a regular expression that goes
  quadratic on crafted input in the Markdown mode. It is fixed only in
  CodeMirror 6, which this project does not use and will not adopt lightly
  (0004). Reaching it means opening a hostile `.md` file that is already in the
  project you opened, and the result is a frozen browser tab, not code
  execution or a leak. OSV records it against CodeMirror's commits up to 5.65.20
  only, so asking about 5.65.21 returns nothing, but its `markdown.js` is
  byte-identical to the affected one. The vendored version and its licence are
  in `THIRD_PARTY_NOTICES.md`.

## Reporting

Open a private security advisory on the GitHub repository, or write to its
owner directly. Please include the request that reproduces
the problem. Rust dependencies are checked against the RustSec database, and the
vendored frontend libraries against OSV (`scripts/audit_vendored.py`), on every
push, every Monday, and locally by `./scripts/check.sh`.
