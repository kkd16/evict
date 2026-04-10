# evict

Kill whatever the fuck is using your port.

A CLI tool that finds and kills processes listening on specified ports.

## Install

```sh
# Quick install
curl -fsSL https://raw.githubusercontent.com/kkd16/evict/main/install.sh | sh

# Homebrew
brew install kkd16/tap/evict

# Or grab a binary from releases
```

## Usage

```sh
evict 3000
evict 3000 8080 5432
sudo evict 80
```

## What it does

1. Finds the process listening on the port (`lsof` on macOS, `ss`/`fuser` on Linux)
2. Insults it
3. Sends `SIGTERM`, waits 500ms
4. If still alive, sends `SIGKILL` with an escalation message
5. Confirms the kill or suggests `sudo`

## License

MIT
