# Question Desk

A small desktop app (Tauri) where a ministry worker logs the hard questions they are asked, stores them locally, and gets an AI-drafted first response plus a "verify these" checklist. A Python script turns the saved questions into a weekly summary and a CSV.

> Training project — use invented questions and askers only. Never put real pastoral content in this repo.

## What you need

The tools from the setup guide: Git, Node.js (18+), Rust (via rustup), the Tauri OS prerequisites for your platform, and Python 3.

## Run it (from a fresh clone)

Same three commands on macOS, Windows and Linux:

```
git clone <your-repo-url> question-desk
cd question-desk
npm install
npm run tauri dev
```

The first run compiles the Rust side and takes a few minutes. A "Question Desk" window opens.

## Set the API key (optional — only needed for Draft)

```
cp .env.example .env        # Windows PowerShell: copy .env.example .env
```

Open `.env` and paste your key after `ANTHROPIC_API_KEY=`, then restart `npm run tauri dev`. `.env` is git-ignored. **Never commit or screenshot the key.**

Without a key the app still works; **Draft** shows "No API key found…" instead of crashing.

Optional: set `ANTHROPIC_MODEL=` in `.env` to override the default model name (check current names at https://docs.claude.com).

## Where your data lives

The app saves one file, `questions.json`:

| OS | Path |
| --- | --- |
| macOS | `~/Library/Application Support/africa.apologetics.questiondesk/questions.json` |
| Windows | `%APPDATA%\africa.apologetics.questiondesk\questions.json` |
| Linux | `~/.local/share/africa.apologetics.questiondesk/questions.json` |

## Weekly report

```
python3 scripts/report.py --days 7
```

Prints how many questions were logged and drafted plus the top tags, and writes `report_YYYY-MM-DD.csv` (columns: `id, asked_at, asker, question, tags, has_draft`) in the current folder. Needs no `pip install`. On Windows use `python` if `python3` is not found.

Options: `--file PATH` (use a different questions.json), `--out PATH`, `--top N`. If the data file is missing the script exits with a message telling you what to do.

## Tests

```
cd src-tauri && cargo test
```

## Known limitations

- Single user, single machine; no sync or backup (copy `questions.json` yourself).
- Drafting needs internet and a paid API key; there is no offline queue.
- Redrafting replaces the previous draft.
- Tags are free text; the report treats them case-insensitively.
- The app is unsigned, so installers will trigger OS security warnings.

## Layout

```
src/            frontend (vanilla TypeScript + one stylesheet)
src-tauri/      Rust: store.rs (JSON file), ai.rs (API call + parsing), lib.rs (commands)
scripts/        report.py
docs/           decisions.md
```
