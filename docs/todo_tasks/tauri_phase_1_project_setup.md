# Tauri Phase 1: Project Setup

**Status:** Not Started
**Dependencies:** None
**Output:** Tauri v2 project skeleton that compiles and opens a window showing the Svelte frontend

---

## Purpose

Initialize a Tauri v2 project with Rust backend and the existing Svelte frontend. The app should compile and open a window — no backend logic yet, just the shell.

---

## Steps

### 1. Remove Python backend

Delete the Python-specific files (the frontend and docs stay):
- `src/` directory (Python backend)
- `pyproject.toml`
- `tests/` directory

Keep: `frontend/`, `docs/`, `themes/`, `CLAUDE.md`, `README.md`, `title.art`, `.gitignore`

### 2. Initialize Tauri

```bash
cargo install create-tauri-app
# Or manually create src-tauri/ with correct structure
```

Create `src-tauri/Cargo.toml`:

```toml
[package]
name = "veil"
version = "0.1.0"
edition = "2021"

[dependencies]
tauri = { version = "2", features = [] }
tauri-build = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

Create `src-tauri/build.rs`:

```rust
fn main() {
    tauri_build::build()
}
```

Create `src-tauri/tauri.conf.json`:

```json
{
  "build": {
    "beforeDevCommand": "cd ../frontend && npm run dev",
    "beforeBuildCommand": "cd ../frontend && npm run build",
    "devUrl": "http://localhost:5173",
    "frontendDist": "../frontend/dist"
  },
  "app": {
    "title": "Veil",
    "width": 900,
    "height": 650,
    "security": {
      "csp": null
    }
  },
  "identifier": "com.veil.app"
}
```

Create `src-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running Veil");
}
```

Create `src-tauri/src/lib.rs`:

```rust
// Module declarations will go here as we build each component
```

### 3. Update frontend for Tauri

Install Tauri API package in the frontend:

```bash
cd frontend && npm install @tauri-apps/api@2
```

The frontend should still work as a standalone web app in dev mode. Tauri-specific code will be added in Phase 7.

### 4. Update .gitignore

Add Rust-specific entries:

```
# Rust
target/
Cargo.lock
```

### 5. Update CLAUDE.md

Update to reflect the Rust + Tauri stack.

### 6. Verify

- `cd src-tauri && cargo build` compiles without errors
- `cargo tauri dev` opens a window showing the Svelte frontend
- The existing Svelte dev server still works independently

---

## Acceptance Criteria

- [ ] Python backend files removed from this branch
- [ ] `src-tauri/` directory created with valid Tauri v2 config
- [ ] `cargo build` in `src-tauri/` compiles successfully
- [ ] Rust module structure scaffolded (empty `mod.rs` files for crypto, identity, bridges, envelope, app)
- [ ] Frontend has `@tauri-apps/api` installed
- [ ] `.gitignore` updated for Rust
- [ ] `CLAUDE.md` updated for Rust + Tauri stack
