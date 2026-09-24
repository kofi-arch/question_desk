# Decisions

**Vanilla TypeScript, not React.** The UI is one form and one list. A framework would add a build-time and learning cost bigger than the app. Rejected: React. Revisit if the UI grows tabs or filters. All user text is set with `textContent`, never `innerHTML`, so pasted text can't inject markup.

**One JSON file, not SQLite.** The brief asks for it, and it lets `report.py` read the same data with only the standard library. Writes go to a temp file then rename, so a crash mid-save can't truncate the data, and a mutex serialises read-modify-write. Rejected: SQLite (better for search and scale, but needs a Rust crate and a Python reader). A corrupt file is reported, never overwritten.

**The API call lives in Rust, not the frontend.** The key stays in the native process and never reaches the webview, where devtools or a bundle could expose it. Rejected: calling from TypeScript. The key is read from the environment / a git-ignored `.env`; if it is missing the command returns a plain-language error.

**Ask for JSON, then parse defensively.** The prompt requires JSON only, but the parser also tolerates code fences and stray text, caps the checklist at four items, and rejects empty results. On failure nothing is saved and the user is told to retry. The system prompt is its own const so it can be reviewed, and it states that output is a starting point for a human.

**Model name is a default plus an override.** `claude-sonnet-5` is the default, and `ANTHROPIC_MODEL` overrides it, so a retired name is a config change, not a code change. It should be re-checked against docs.claude.com before release.

**Delete uses an inline two-step confirm** instead of `window.confirm`, which behaves differently across webviews and is awkward to test.
