# AdCleanse: Desktop Privacy Audit & Ad-Profile Scrubbing Console
### Prepared by Bored Polymath Studios

AdCleanse is an offline-first, native desktop utility built using the [Tauri](https://tauri.app/) framework (v2) designed to empower users with auditable control over their algorithmic advertising personas on Meta platforms. Commercial social networks obscure the tracking schemas, partner uploads, and behavioral categories linked to user accounts. AdCleanse acts as an isolated privacy microscope, extracting ad interest categories, measuring tracking vector drift over time, and executing automated topic-scrubbing actions directly on Meta's preference endpoints without routing sensitive user data through proprietary cloud infrastructure.

---

## Key Architectural Principles

1. **Zero-Telemetry Guarantee**: No external telemetry servers, crash reporters, analytics engines, or tracking beacons. All outbound HTTP requests route strictly to Meta endpoints or user-configured encrypted sync relays.
2. **Native Memory & Credential Security**: Session tokens are isolated within native operating system keyrings (macOS Keychain, Windows Credential Manager, Linux Secret Service). RAM buffers holding sensitive payloads are zeroized upon drop.
3. **Encrypted Local Persistence**: Differential audit snapshots, partner upload registries, and blocklists persist locally in an encrypted SQLite database leveraging SQLCipher (AES-256).
4. **Lightweight Resource Footprint**: Passive system-tray daemon memory consumption remains strictly under 35 MB RSS; cold start initializes in under 800 ms.
5. **Humanized Interaction & Rate-Limit Resilience**: Requests emulate desktop browser cadences with randomized delays and exponential jitter backoff, protecting accounts against automated throttling.

---

## Project Structure

```text
adcleanse/
├── .gitignore                    # Ensures build artifacts, keys, and private /docs/ stay ignored
├── adcleanse.config.json         # Desktop client settings & threshold configuration
├── docs/                         # Private SDLC checklists & internal planning (gitignored)
│   └── adcleanse-master-sdlc-checklist.md
├── src-tauri/                    # Native compiled Rust core (Tauri v2)
│   ├── Cargo.toml                # Rust crate dependencies and build parameters
│   ├── tauri.conf.json           # Tauri v2 window, security CSP, and bundle definitions
│   ├── build.rs                  # Build script invoking tauri-build
│   ├── Entitlements.plist        # macOS Hardened Runtime security entitlements
│   ├── deny.toml                 # Supply chain security & license compliance policies
│   ├── capabilities/             # Tauri v2 security capability manifests
│   │   └── default.json
│   ├── icons/                    # Multi-resolution application & tray icons
│   └── src/
│       ├── main.rs               # Application entry point
│       ├── lib.rs                # Core Tauri builder & IPC registration
│       ├── error.rs              # Unified error types across subsystems
│       ├── commands/             # Strongly typed IPC command handlers
│       ├── auth/                 # Sandboxed login webview & session management
│       ├── keyring/              # Platform credential store abstraction
│       ├── audit/                # Ad topic extraction & differential engine
│       ├── scrubber/             # Automated rule enforcement & mutation dispatch
│       ├── storage/              # SQLCipher encrypted SQLite persistence
│       ├── tray/                 # System tray daemon & context menus
│       ├── hotkey/               # Global shortcut listener & Spotlight panel
│       ├── network/              # Zero-telemetry client & Network Ledger
│       └── notifications/        # Native OS desktop notification dispatch
└── ui/                           # Presentation layer (HTML5 / Vanilla CSS / JS)
    ├── index.html                # Responsive desktop dashboard layout
    ├── css/
    │   └── index.css             # Craft-focused dark mode design system
    ├── js/
    │   └── app.js                # Frontend IPC bridge & reactive view controllers
    └── assets/                   # SVG icons & branding graphics
```

---

## Development Prerequisites

* **Rust Toolchain**: `rustc` and `cargo` (1.75+ or newer, 2021 edition).
* **Tauri CLI**: `cargo install tauri-cli --version "^2.0"`.
* **Platform Dependencies**:
  * **macOS**: Xcode Command Line Tools.
  * **Windows**: Visual Studio 2022 C++ Build Tools & WebView2 runtime.
  * **Linux**: `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `libsecret-1-dev`.

---

## Running and Building

### Standalone UI Preview (Browser Mode)
Because the frontend presentation layer utilizes vanilla web standards and includes an embedded IPC mock dispatcher, you can preview the UI directly in any modern browser:

```bash
open ui/index.html
```

### Tauri Desktop Development Mode
To launch the full native desktop shell with the Rust backend:

```bash
cargo tauri dev
```

### Production Release Bundling
To generate native signed installers:

```bash
cargo tauri build
```

---

## Licensing
Dual-licensed under either of:
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

---

## Trademark & Legal Disclaimer
**AdCleanse** is an independent, local open-source privacy audit utility authored by Bored Polymath Studios. AdCleanse is not affiliated with, sponsored by, authorized by, or endorsed by Meta Platforms, Inc., Facebook, Instagram, or any of their affiliates or subsidiaries. All product names, trademarks, and registered trademarks cited in documentation or UI are property of their respective holders and are used solely for identification and interoperability purposes.
