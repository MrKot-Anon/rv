# RV

<img src="assets/app-icon.png" width="128" height="128" alt="RV app icon" align="right">

A native desktop VNC viewer written in Rust with [GPUI](https://www.gpui.rs/). Direct RFB (RFC 6143) connections, address-book chrome, and a session toolbar inspired by RealVNC Classic Viewer / Connect Viewer — original branding, not a RealVNC product.

## Features

- Address book with search, labels, list/grid views, desktop previews, and recents
- Light / dark / system appearance
- Session window with pinned or auto-hide toolbar, F8 menu, and connection info
- Fit / 1:1 (scrollable) / stretch scaling and full screen
- Mouse (left/middle/right, vertical + horizontal wheel), keyboard, Ctrl+Alt+Del and extra keys (Ctrl/Alt/Win/Tab/Esc/Caps — Caps toggles the remote caps-lock)
- Reconnect from the disconnect / error overlay
- Clipboard sync (Latin-1, per RFB)
- VNC Auth, Tight / ZRLE / TRLE / Raw encodings
- Mac Screen Sharing / Apple Remote Desktop (ARD) login using the **Username** and **Password** fields in connection properties. ARD encrypts credentials, but requires no legacy VNC password setting on the Mac. Use **Let server choose**; **Always on** still requires VeNCrypt session TLS.
- VeNCrypt TLS: `TLSVnc` / `TLSNone` (anonymous TLS, TigerVNC's default) via OpenSSL, `X509Vnc` / `X509None` via rustls with WebPKI roots. `Let server choose` picks encryption automatically when the server offers nothing else
- Optional saved passwords in local files under the user's application data directory

## Build

```bash
cargo run -p rv-app --release
```

Connect to a local server such as TigerVNC, TightVNC, x11vnc, QEMU, or `rustvncserver` on `127.0.0.1:5900`, or pass a target on the command line:

```bash
rv 10.0.0.8          # port 5900
rv pi.local:1        # display 1 → port 5901 (numbers below 100 are displays)
rv pi.local::5901    # explicit port
rv [2001:db8::1]:2   # IPv6
```

## Keyboard shortcuts

| Address book | | Session | |
| --- | --- | --- | --- |
| ⌘N | New connection | F8 | Session menu |
| ⌘F | Search | ⇧⌘F | Full screen |
| ⌘L | Toggle list / grid | ⌘W | Close window |
| ⌘I | Properties | | |
| ⌘D | Duplicate | | |
| ⌘B | Toggle sidebar | | |
| ⌘, | Preferences | | |
| ↩ / ⌫ | Connect / delete selected | | |

Ctrl / Alt / ⌘ are forwarded to the remote desktop while a session has focus.

## Tests

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

CI runs these checks and builds the app on macOS, Linux, and Windows, each on
x86_64 and ARM64 native runners. It runs for pull requests and pushes to `main`
or `master`, and can also be started manually.

The UI tests render and dispatch keyboard/mouse events through GPUI's test
platform, without native windows, a display server, or a GPU. They use temporary
address books and an in-memory VNC peer, covering form validation and saving,
busy controls, search, keyboard forwarding, pointer mapping, and cursor restoration.
Run just these tests with:

```bash
cargo test -p rv-app --locked offscreen_
```

A mock RFB server for manual testing lives in `crates/rv-session/examples/mock_server.rs`:

```bash
RV_MOCK_SIZE=1280x800 RV_MOCK_RECTS=8 cargo run -p rv-session --example mock_server 127.0.0.1:5999
RV_DATA_DIR=/tmp/rv-scratch cargo run -p rv-app -- 127.0.0.1:5999
```

For pointer diagnostics, use `RV_MOCK_SIZE=1920x1080 RV_MOCK_LOG_MOTION=1` on
the server. It prints every received pointer position and draws a white
crosshair at that position in the framebuffer. Buttons 1, 8, and 16 in the
printed mask correspond to left click, wheel up, and wheel down.

`RV_DATA_DIR` overrides the address-book location (default: the platform data directory).

RV detects the extra framebuffer rows used by x11vnc `-ncache` when the server
advertises a very tall exact multiple of a common desktop size. It displays
only the top desktop while retaining the cache rows for VNC CopyRect updates.

To test Mac authentication locally:

```sh
RV_MOCK_ARD=1 cargo run -p rv-session --example mock_server 127.0.0.1:5999
```

Connect to `127.0.0.1:5999` with username `test-user` and password `test-password`
(or set `RV_MOCK_USERNAME` / `RV_MOCK_PASSWORD` on the server). The mock offers
`ARD, 33, 36, 35`, decrypts and validates both credentials, and rejects incorrect
logins. ARD mode takes precedence over `RV_MOCK_TLS`. Automated TCP tests cover
accepted/rejected logins, missing credentials, desktop frames, and input after
authentication; malformed DH parameters and credential byte limits have unit tests.

## macOS app bundle

Package a release build with:

```bash
cargo build --release -p rv-app --locked
python3 scripts/package-macos.py \
  --binary target/release/rv --output target/release/RV.app
```

Saved passwords are stored as plain text in separate files inside
`credentials/` under the app's data directory. On Unix, the directory is mode
`0700` and each file is mode `0600`. Existing Keychain passwords are not imported;
enter them again to save them locally.

## Layout

| Crate | Role |
| --- | --- |
| `rv-core` | Connection models, address book, keysyms, prefs |
| `rv-session` | Tokio RFB session, framebuffer, VeNCrypt |
| `rv-app` | GPUI UI |

macOS is the primary target. Linux and Windows should compile via GPUI.

## License

[MIT](LICENSE) © 2026 Max Lv
