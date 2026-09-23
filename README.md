# Personal Rhythm Assistant

macOS-first, local-first desktop companion for healthier work rhythm.

See:

- `docs/IMPLEMENTATION.md`
- `docs/MILESTONES.md`

## Development

Requires Node 22+, Rust (stable) and Xcode command line tools.

```sh
npm install
npm run tauri dev     # run the desktop app
npm run check         # typecheck + lint + format + frontend tests + Rust tests
```

Frontend tests live in `tests/frontend`, Rust tests in `tests/core` (registered in `src-tauri/Cargo.toml`).
