# Sidfrey Router (headless)

Local HTTP router for custom `!bangs`. Similar to [Sidfrey.xyz](https://sidfrey.xyz) ([source](https://github.com/jamesondh/sidfreyxyz)), it parses queries like `hello world !g` and 302-redirects to the right engine. Runs on `localhost`. No cloud. Default engine: **Perplexity**.

## Quick description

* Input: `GET /?q=<query>`
* Output: `302 Location: <engine URL>`
* Prefix or suffix bangs supported (`!g foo` or `foo !g`).
* Unknown or bare `!` fall back to DuckDuckGo.
* No HTTPS needed on `localhost`. Binds to `127.0.0.1` only.

## Running

```bash
# clone and run
cargo run --release

# optional: choose port
SIDFREY_PORT=7777 cargo run --release
```

Test:

```bash
curl -i 'http://127.0.0.1:7777/?q=hello%20world%20!g'
# HTTP/1.1 302 Found
# Location: https://www.google.com/search?q=hello%20world
```

### Browser setup (Chromium: Chrome/Brave/Edge)

Settings → Search engine → Manage search engines → **Add**:

* Name: `Sidfrey Router`
* Shortcut: `sf`  (or leave blank)
* URL: `http://127.0.0.1:7777/?q=%s`

You can set it as default if you want to type directly in the omnibox without a keyword.

Examples to try in the omnibox:

* `hello world !g` → Google
* `!gh rust ownership` → GitHub
* `quantum gates` → Perplexity (default)

## Building

```bash
cargo build --release
install -Dm755 target/release/sidfrey-router ~/.local/bin/sidfrey-router   # Linux
# or copy to /usr/local/bin on macOS:
sudo install -m755 target/release/sidfrey-router /usr/local/bin/sidfrey-router
```

### Building for Android (64-bit ARM)

**Prerequisites:**
1. Install Android NDK:
   ```bash
   # Direct download (no Android Studio needed)
   wget https://dl.google.com/android/repository/android-ndk-r26b-darwin.zip  # macOS
   # or
   wget https://dl.google.com/android/repository/android-ndk-r26b-linux.zip   # Linux
   
   unzip android-ndk-r26b-*.zip
   export ANDROID_NDK_HOME=$PWD/android-ndk-r26b
   ```

2. Install Rust target and cargo-ndk:
   ```bash
   rustup target add aarch64-linux-android
   cargo install cargo-ndk
   ```

3. Build for Android:
   ```bash
   cargo ndk -t arm64-v8a build --release
   # Binary will be at: target/aarch64-linux-android/release/sidfrey-router
   ```

**Note:** This builds for 64-bit ARM Android devices (most modern phones). The binary will be ~2-5MB.

## Run as a background service

### Android (Termux with Termux:Boot)

**Setup Termux:**
1. Install [Termux](https://f-droid.org/packages/com.termux/) from F-Droid (not Google Play)
2. Install [Termux:Boot](https://f-droid.org/packages/com.termux.boot/) addon from F-Droid
3. Open Termux:Boot once to enable boot permission

**Transfer and run the binary:**
```bash
# In Termux, grant storage permission
termux-setup-storage

# Copy binary from Downloads to Termux home (Downloads folder doesn't allow execution)
cp ~/storage/downloads/sidfrey-router ~/
chmod +x ~/sidfrey-router

# Test run
./sidfrey-router
# or with custom port
SIDFREY_PORT=8080 ./sidfrey-router
```

**Auto-start on boot with Termux:Boot:**
```bash
# Create boot directory
mkdir -p ~/.termux/boot

# Create startup script
cat > ~/.termux/boot/start-sidfrey.sh << 'EOF'
#!/data/data/com.termux/files/usr/bin/sh
# Start Sidfrey Router on boot
cd ~
SIDFREY_PORT=7777 ./sidfrey-router > sidfrey.log 2>&1 &
EOF

# Make executable
chmod +x ~/.termux/boot/start-sidfrey.sh

# Reboot device to test, or manually run the script
~/.termux/boot/start-sidfrey.sh
```

**Managing the service:**
```bash
# Check if running
pgrep sidfrey-router

# View logs
tail -f ~/sidfrey.log

# Stop the server
pkill sidfrey-router
```

**Browser setup on Android:**
- Chrome/Brave: Settings → Search engine → Add search engine
- URL: `http://127.0.0.1:7777/?q=%s`
- Set as default if desired

**Note:** The server will survive Termux app closure but not device reboots (unless using Termux:Boot).

### Linux (systemd user service)

Create `~/.config/systemd/user/sidfrey-router.service`:

```ini
[Unit]
Description=Sidfrey local bang router

[Service]
ExecStart=%h/.local/bin/sidfrey-router
Environment=SIDFREY_PORT=7777
Restart=on-failure
# Bind to loopback only (default). If you change the binary, keep it.
# ExecStart can be /usr/local/bin/sidfrey-router if you installed there.

[Install]
WantedBy=default.target
```

Enable and start:

```bash
systemctl --user daemon-reload
systemctl --user enable --now sidfrey-router
systemctl --user status sidfrey-router
```

Logs:

```bash
journalctl --user -u sidfrey-router -f
```

### macOS (launchd, per-user agent)

Create `~/Library/LaunchAgents/com.sidfrey.router.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>com.sidfrey.router</string>
  <key>ProgramArguments</key>
  <array>
    <string>/usr/local/bin/sidfrey-router</string>
  </array>
  <key>EnvironmentVariables</key>
  <dict>
    <key>SIDFREY_PORT</key><string>7777</string>
  </dict>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>StandardOutPath</key><string>/tmp/sidfrey-router.out</string>
  <key>StandardErrorPath</key><string>/tmp/sidfrey-router.err</string>
</dict>
</plist>
```

Load and start:

```bash
launchctl load ~/Library/LaunchAgents/com.sidfrey.router.plist
launchctl start com.sidfrey.router
launchctl list | grep com.sidfrey.router
```

Update after edits:

```bash
launchctl unload ~/Library/LaunchAgents/com.sidfrey.router.plist
launchctl load ~/Library/LaunchAgents/com.sidfrey.router.plist
```

## Usage notes

* Default engine is Perplexity. Change it in code if desired.
* Path override also works: `/google?q=foo` or `/perplexity?q=bar`.
* Keep it bound to `127.0.0.1` for safety. If you expose it, you own the risk.

## Troubleshooting

* Port in use → set `SIDFREY_PORT` to a free port and update the browser URL template.
* No redirects → check service logs, verify the browser is pointing to `http://127.0.0.1:<port>/?q=%s`.
* Bang not recognized → add or edit the mapping in code and rebuild.

## Uninstall

```bash
# macOS
launchctl unload ~/Library/LaunchAgents/com.sidfrey.router.plist
rm ~/Library/LaunchAgents/com.sidfrey.router.plist
sudo rm -f /usr/local/bin/sidfrey-router

# Linux
systemctl --user disable --now sidfrey-router
rm ~/.config/systemd/user/sidfrey-router.service
rm ~/.local/bin/sidfrey-router
systemctl --user daemon-reload

# Android (Termux)
pkill sidfrey-router
rm ~/.termux/boot/start-sidfrey.sh
rm ~/sidfrey-router
rm ~/sidfrey.log
```
