# Code Context

## Files Retrieved
1. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/README.md` (lines 22-194) - product claims, requirements, install/CLI split, lifecycle/ingress usage.
2. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/DEVELOPMENT.md` (lines 1-130) - local prerequisites, capabilities, config keys.
3. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/go.mod` (lines 1-142) - Go version and dependency profile.
4. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/openapi.yaml` (lines 1-160, 528-688, 2042-4191) - API version, create-instance shape, endpoint surface.
5. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/cmd/api/main.go` (lines 134-685) - server bootstrap, managers, middleware, registry, metrics, background controllers.
6. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/cmd/api/wire.go` (lines 29-70) and `cmd/api/wire_gen.go` (lines 37-109) - dependency graph.
7. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/cmd/api/config/config.go` (lines 245-639) - configuration schema, defaults, env overrides, validation.
8. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/providers/providers.go` (lines 39-427) - manager construction and cross-module wiring.
9. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/paths/paths.go` (lines 1-411) - local filesystem layout for all state.
10. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/instances/types.go` (lines 1-343) and `lib/instances/manager.go` (lines 28-180) - instance state, persisted metadata, manager API.
11. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/instances/create.go` (lines 73-932) - OCI image → disks/config/network/devices → hypervisor boot flow.
12. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/instances/standby.go` (lines 23-417), `restore.go` (lines 25-589), `snapshot.go` (lines 48-240), `fork.go` (lines 28-240) - standby/restore/snapshot/fork maturity.
13. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/hypervisor/hypervisor.go` (lines 29-326) - hypervisor abstraction, capabilities, vsock.
14. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/instances/hypervisor_linux.go` (lines 1-33), `lib/instances/hypervisor_darwin.go` (lines 1-28), `cmd/api/hypervisor_check_linux.go` (lines 1-29), `cmd/api/hypervisor_check_darwin.go` (lines 1-31) - platform registration/access checks.
15. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/hypervisor/cloudhypervisor/cloudhypervisor.go` (lines 1-296), `firecracker/firecracker.go` (lines 1-220), `qemu/qemu.go` (lines 1-212), `vz/client.go` (lines 1-130), `vz/starter.go` (lines 1-282) - per-hypervisor capabilities.
16. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/vmm/binaries_linux.go` (lines 1-110), `lib/vmm/binaries_darwin.go` (lines 1-36), `lib/ingress/binaries_linux.go` (lines 1-131), `lib/ingress/binaries_darwin.go` (lines 1-33) - embedded binary strategy and macOS gaps.
17. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/system/manager.go` (lines 1-99), `lib/system/init/main.go` (lines 1-137), `lib/system/guest_agent/exec.go` (lines 1-260), `lib/guest/guest.proto` (lines 1-171) - kernel/initrd, guest init, exec/cp/stat agent model.
18. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/images/manager.go` (lines 32-359), `lib/images/disk.go` (lines 1-275) - OCI pull/export, EROFS/ext4 disk conversion.
19. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/network/manager.go` (lines 16-203), `bridge_linux.go` (lines 1-240), `bridge_darwin.go` (lines 1-81), `allocate.go` (lines 18-260) - network model.
20. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/volumes/manager.go` (lines 19-220), `lib/volumes/types.go` (lines 1-57) - persistent block volume lifecycle.
21. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/ingress/manager.go` (lines 37-300) - Caddy + DNS ingress subsystem.
22. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/builds/manager.go` (lines 35-338, 372-792), `lib/builds/images/generic/Dockerfile` (lines 1-58), `lib/builds/builder_agent/main.go` (lines 1-280) - source-build pipeline.
23. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/registry/registry.go` (lines 1-260) - embedded OCI registry and conversion trigger.
24. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/resources/resource.go` (lines 1-280) - host resource discovery/admission model.
25. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/devices/types.go` (lines 1-127), `lib/devices/manager.go` (lines 40-300), `discovery_linux.go` (lines 1-180), `discovery_darwin.go` (lines 1-56) - GPU/PCI/vGPU support and macOS limits.
26. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/autostandby/types.go` (lines 1-76), `controller.go` (lines 1-260), `lib/providers/auto_standby_linux.go` (lines 1-126), `auto_standby_unsupported.go` (lines 1-17) - auto-standby platform support.
27. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/egressproxy/service.go` (lines 1-260), `lib/instances/egress_proxy.go` (lines 1-322), `lib/system/init/egress_proxy.go` (lines 1-46) - mediated egress/credential injection.
28. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/lib/middleware/oapi_auth.go` (lines 56-94, 309-513), `lib/scopes/scopes.go` (lines 1-274), `cmd/gen-jwt/main.go` (lines 1-101) - JWT auth/scopes/token tool.
29. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/config.example.yaml` (lines 1-155), `config.example.darwin.yaml` (lines 1-171), `cli.example.yaml` (lines 1-20) - operator-facing config and platform notes.
30. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/Makefile` (lines 39-423) - build/test/release prep targets.
31. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/.github/workflows/test.yml` (lines 1-263), `release.yml` (lines 1-36), `deploy.yml` (lines 1-78), `semgrep.yml` (lines 1-17), `vuln-remediation.yml` (lines 1-17) - CI/release/security workflows.
32. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/.goreleaser.yaml` (lines 1-84) and `scripts/install.sh` (lines 69-123, 328-391, 538-790) - release artifacts/install packaging.
33. `/Users/User/.cache/checkouts/github.com/kernel/hypeman/deploy/README.md` (lines 1-102), `deploy/aws/cloudformation/template.yaml` (lines 76-102, 205-395, 469-549), `deploy/aws/cloudformation/template_test.go` (lines 1-220) - AWS one-host deployment assets.

## Key Code

```go
// cmd/api/wire.go:29-47
// One process owns all managers and the API service.
type application struct {
  ImageManager images.Manager
  SystemManager system.Manager
  NetworkManager network.Manager
  DeviceManager devices.Manager
  InstanceManager instances.Manager
  VolumeManager volumes.Manager
  IngressManager ingress.Manager
  BuildManager builds.Manager
  ResourceManager *resources.Manager
  Registry *registry.Registry
  ApiService *api.ApiService
}
```

```go
// lib/instances/manager.go:28-70
// The instance manager is the core runtime API.
type Manager interface {
  CreateInstance(ctx context.Context, req CreateInstanceRequest) (*Instance, error)
  StandbyInstance(ctx context.Context, id string, req StandbyInstanceRequest) (*Instance, error)
  RestoreInstance(ctx context.Context, id string) (*Instance, error)
  ForkInstance(ctx context.Context, id string, req ForkInstanceRequest) (*Instance, error)
  StreamInstanceLogs(...)
  AttachVolume(...)
  GetVsockDialer(ctx context.Context, instanceID string) (hypervisor.VsockDialer, error)
}
```

```go
// lib/hypervisor/hypervisor.go:96-271
// Hypervisors implement a common start/control/capabilities interface.
type VMStarter interface { StartVM(...); RestoreVM(...); PrepareFork(...) }
type Hypervisor interface { DeleteVM(...); Pause(...); Resume(...); Snapshot(...); ResizeMemory(...); Capabilities() Capabilities }
type Capabilities struct { SupportsSnapshot, SupportsHotplugMemory, SupportsVsock, SupportsGPUPassthrough bool }
```

```yaml
# openapi.yaml:528-637
CreateInstanceRequest:
  required: [name, image]
  properties:
    size, hotplug_size, overlay_size, vcpus, env, tags, network,
    devices, gpu, volumes, hypervisor, snapshot_policy,
    auto_standby, health_check, restart_policy, entrypoint, cmd
```

```go
// lib/paths/paths.go:15-411
// All state is local filesystem state under data_dir.
images/<repo>/<digest>/rootfs.{erofs,ext4}
guests/<id>/metadata.json, overlay.raw, config.ext4, logs/, snapshots/
volumes/<id>/data.raw, metadata.json
snapshots/<snapshotID>/snapshot.json, guest/
ingresses/<id>.json, builds/<id>/metadata.json
```

## Architecture

- **Shape:** Go monolith API server. `cmd/api/main.go` loads YAML/env config, initializes OpenTelemetry, constructs managers via Wire, verifies KVM/VZ, ensures kernels/initrd, initializes network/device/ingress, mounts generated OpenAPI routes, custom WebSocket exec/cp routes, and `/v2` OCI registry endpoints (lines 134-685).
- **API surface:** OpenAPI 3.1 API v0.2.0 with resources for health/resources/images/instances/snapshots/volumes/devices/ingresses/builds (not templates/workspaces/users). Exec and cp are custom WebSocket routes outside OpenAPI.
- **State model:** No database found. Managers persist JSON metadata and raw disk images under one `data_dir`; resource admission and recovery scan local files and host state.
- **Instance boot flow:** `CreateInstance` validates/auto-pulls OCI image, reserves resources, allocates network, creates overlay/config disks, attaches volumes/devices/vGPU, writes metadata, builds `hypervisor.VMConfig`, starts the selected VMM, and waits for guest boot markers/guest-agent readiness (`lib/instances/create.go:73-932`).
- **Guest model:** OCI image rootfs becomes read-only EROFS on Linux or ext4 on macOS. A Hypeman init binary mounts rootfs/overlay/config/volumes, supports exec or systemd mode, injects a guest-agent, and exposes gRPC over vsock for exec/cp/stat/shutdown/network reconfigure.
- **Hypervisors/platforms:** Linux registers Cloud Hypervisor, Firecracker, QEMU; macOS registers VZ plus stubs for unsupported CH/QEMU paths. Linux requires `/dev/kvm`; macOS check requires arm64 and Virtualization.framework. Capabilities differ: CH has snapshot/hotplug/GPU/disk I/O; Firecracker has snapshots/balloon/vsock but no GPU/hotplug; QEMU supports GPU/vsock/snapshots but not hotplug; VZ supports snapshots only on arm64 and lacks GPU/disk I/O limits.
- **Networking:** Linux creates a bridge/TAPs, iptables NAT/forwarding, and HTB traffic shaping. macOS bridge/TAP/iptables/HTB functions are no-ops and VZ uses NAT/DHCP (`192.168.64.0/24`).
- **Ingress:** Starts Caddy plus an internal DNS server that resolves instances dynamically. TLS uses DNS-01/Cloudflare and allowed-domain checks.
- **Builds:** Source bundle becomes temporary volumes; a builder VM runs BuildKit and pushes to the embedded `/v2` registry; registry manifest PUT triggers import/conversion to runnable disk image. Default builder image is built from an embedded Dockerfile using the host Docker socket unless `build.builder_image` is configured.
- **Cloud deploy:** Maintained AWS CloudFormation quickstart creates a single EC2 host with nested virtualization, an encrypted XFS data EBS volume, CIDR-restricted API port, SSM access, and runs `scripts/install.sh` during UserData.

## Maturity Signals

- **Codebase size:** 509 Go files, 184 `*_test.go` files, 56 Go packages observed via `git ls-files`/`go list`.
- **Tests/CI:** GitHub Actions runs Linux tests on self-hosted `linux,x64,kvm`, macOS tests on self-hosted `macos,arm64`, prewarms images, builds OpenAPI code, checks gofmt, installs UFFD pager systemd template, and has a macOS install E2E job (`.github/workflows/test.yml:1-263`).
- **Integration coverage:** Includes KVM/systemd-mode and vGPU integration tests, many unit tests across managers, and deployment template tests.
- **Security hygiene:** Semgrep PR workflow and scheduled vulnerability-remediation workflow exist (`semgrep.yml:1-17`, `vuln-remediation.yml:1-17`). Auth has route scopes and registry-scoped builder tokens, but legacy user tokens without `permissions` are still full-access.
- **Release packaging:** Tags trigger GoReleaser on a self-hosted Linux KVM runner (`release.yml:1-36`). GoReleaser builds Linux amd64/arm64 artifacts for `hypeman-api`, `hypeman-token`, and `hypeman-uffd-pager` only (`.goreleaser.yaml:8-49`). Release archives include only LICENSE/README/RELEASES besides binaries.
- **Installer:** `scripts/install.sh` supports Linux and Darwin detection, config generation, systemd/launchd setup, token generation, and separate CLI install from `kernel/hypeman-cli` (`scripts/install.sh:69-123`, `538-790`).
- **Packaging gap:** Installer attempts Darwin release downloads/signing, but `.goreleaser.yaml` currently declares only `goos: linux`; macOS appears supported by source/dev/CI, not release artifacts.
- **Deployment maturity:** AWS deploy assets are validated and published by workflow; CloudFormation is tested and publishes from `main` to S3. It is explicitly a **single EC2 instance** quickstart, not a multi-node control plane.

## Major gaps for a local+cloud Coder-like runtime

1. **No workspace/template/user model.** API primitives are images, instances, volumes, builds, ingresses, snapshots. There are no users/orgs/templates/workspaces/agents/developer sessions comparable to Coder.
2. **No tenant isolation in resource ownership.** JWT `sub` is put in context, and scopes gate actions, but resources are not filtered/owned by user; tokens without `permissions` retain full access.
3. **Single-host architecture.** Local filesystem state, local bridges/TAPs, local VMM sockets, local Caddy, and single-host AWS deployment mean no scheduler, no multi-host placement, no HA, no central DB, and no object-store-backed image/snapshot distribution.
4. **Cloud exposure is minimal.** AWS quickstart exposes `http://<public-ip>:8080` to an allowed CIDR and token auth. Production-grade cloud needs TLS-by-default API, SSO/OIDC, audit logs, key rotation, regional routing, and managed ingress/proxy isolation.
5. **macOS release support is inconsistent.** Code/tests support Darwin arm64 VZ, but release artifacts are Linux-only; Caddy is not embedded on macOS and must be installed separately; macOS lacks GPU passthrough, network/disk I/O limiting, CPU/memory hotplug, and has snapshot constraints.
6. **Builder maturity risk.** Default builder image uses `moby/buildkit:latest` and host Docker socket for preparation; production should pin/supply builder images and reduce Docker daemon coupling.
7. **Coder UX features absent.** Need first-party web UI/dashboard, browser/VS Code/SSH connection flows, workspace port forwarding with auth, devcontainer parsing, dotfiles, per-user home/project volume conventions, and lifecycle policies at workspace/template level.
8. **Snapshot/volume portability.** Snapshot and image data are local raw directories/files and hypervisor-specific; cloud runtime needs export/import, durable backup, cross-host restore/fork, and versioned migrations.

## Start Here

Open `cmd/api/main.go` first. It shows the full runtime lifecycle, manager dependencies, HTTP/registry routing, background controllers, and shutdown behavior. Then jump to `lib/instances/create.go` for the VM boot/data-flow core and `lib/hypervisor/hypervisor.go` for the VMM abstraction.

## Supervisor coordination

No decision needed.
