# smolcoder

`smolcoder` opens a persistent smolvm machine as a Remote SSH coding target.

```bash
smolcoder open --ide code
smolcoder open --ide intellij
```

## What it manages

- Stable per-workspace machine names
- Create/start/update of the backing embedded `smolvm` machine
- Debian/glibc-backed guest with root SSH login
- Localhost SSH port forwarding into the machine
- SSH public key or `authorized_keys` staging
- Isolated `ssh_config` and `known_hosts`
- Local SSH port selection
- Short VS Code user-data paths under `/tmp/smolcoder`
- VS Code Remote-SSH settings:
  - `remote.SSH.enableDynamicForwarding = false`
  - `remote.SSH.useExecServer = false`
- JetBrains Gateway deep links for IntelliJ remote development
- Gateway is restarted on macOS when needed so generated SSH connections are loaded

The guest mounts the current workspace at `/workspace`.

## Usage

```bash
# Open the current directory in VS Code Remote SSH.
smolcoder open --ide code

# Prepare the machine without launching an IDE.
smolcoder ensure

# Print connection details.
smolcoder status

# Open JetBrains Gateway directly into the remote project.
smolcoder open --ide intellij

# Stop or delete the machine.
smolcoder stop
smolcoder delete
```

Useful options:

```bash
smolcoder open --workspace /path/to/repo
smolcoder open --name smolcoder-myrepo
smolcoder open --port 2222
smolcoder open --public-key ~/.ssh/id_ed25519.pub
smolcoder open --authorized-keys /absolute/path/to/authorized_keys
smolcoder open --recreate
smolcoder open --ide intellij --reset-intellij-cache
```

## Requirements

- `ssh` in `PATH`
- `code` in `PATH` for VS Code launches
- A usable SSH public key, or an explicit `--public-key` / `--authorized-keys`
- The guest must be image-backed with a glibc Linux base for JetBrains Remote Development. `smolcoder` creates `debian:bookworm-slim`; recreate older bare/Alpine machines with `smolcoder open --recreate --ide intellij`.

## Troubleshooting

If JetBrains reports `Please try to reinstall the IDE` or a JBR `lib/modules size has changed` error, clear the remote IDE backend cache and retry:

```bash
smolcoder open --ide intellij --reset-intellij-cache
```
