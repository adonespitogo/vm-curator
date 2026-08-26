# vm-curator

A fast and friendly Rust TUI for managing desktop QEMU/KVM virtual machines — with 3D acceleration, GPU and physical-disk passthrough, managed virtual networks, multi-NIC VMs, freeform VM groups, VM import, and 130+ pre-configured OS profiles!

See [CHANGELOG.md](CHANGELOG.md) for release notes.

### Features

**VM Discovery & Organization**
- Automatically scans your VM library for directories containing `launch.sh` scripts
- Hierarchical organization by 16 OS families with emoji icons and 50 subcategories (used automatically until you define your own VM Groups, see below)
- Parses QEMU launch scripts to extract configuration (emulator, memory, CPU, VGA, audio, network, disks)
- Smart categorization with configurable hierarchy patterns
- Live process monitoring — shows running VMs with status indicators
- Search and filter VMs by name

**VM Info Panel**
- The main menu's info panel shows a live overview of the selected VM's actual QEMU configuration — no more digging through `launch.sh` to check what a VM is set up with
- Hardware: architecture, CPU cores/model, memory, machine type, and KVM/UEFI/TPM/3D-acceleration feature flags
- Disks: full path, format, interface, and role (system/firmware/media), plus used/capacity size read live via `qemu-img info` (cached per disk so browsing the VM list stays instant)
- Network topology: every NIC's backend (user/SLIRP, passt, bridge), bridge name, MAC address, and forwarded ports
- ASCII art and your free-form VM notes are still shown alongside it

**VM Groups**
- Organize VMs into freeform groups instead of (or alongside) the automatic OS-family hierarchy — press `g` on the main menu
- Create, rename, delete, and reorder groups (`Shift+J`/`Shift+K`), and manage which VMs belong to each
- New VMs default into a group matching their OS category, or pick a group explicitly (or type a new one) from the Create/Import wizards' review step
- Once any group is defined, the main menu's VM list follows your group order (with a trailing "Ungrouped" section); clearing all groups falls back to the automatic hierarchy
- Persisted to `~/.config/vm-curator/groups.toml`, auto-pruned of stale VM ids as VMs are deleted or the library is rescanned

**VM Creation Wizard**
- 5-step guided wizard for creating new VMs
- 130+ pre-configured OS profiles with optimal QEMU settings (Windows, macOS, Linux, BSD, Unix, retro, and more)
- Automatic UEFI firmware detection across Linux distributions (Arch, Debian, Fedora, NixOS, etc.)
- ISO file browser for selecting installation media
- Configurable disk size/format, memory, CPU cores, and QEMU options with direct text editing and size suffixes (e.g., "8GB")
- Use existing disk images (copy or move) instead of creating new ones
- Pass a whole physical disk (NVMe/SATA/USB) through as the boot device instead of using a disk image
- Support for custom OS entries with user metadata

**VM Import Wizard**
- Import existing VMs from libvirt (virsh) XML configurations and Quickemu `.conf` files
- 5-step guided import: select source, choose VM, review compatibility warnings, configure disk handling, review and import
- Automatic OS profile detection from imported configurations
- Disk handling options: symlink, copy, or move existing disk images

**GPU Passthrough**
- **Single-GPU passthrough**: Pass your only GPU to a VM (requires TTY, stops display manager)
- **Guest driver compatibility**: Per-VM vBIOS ROM (`romfile=`) and Hide-KVM (`kvm=off`) options for AMD/NVIDIA Windows driver quirks
- **Multi-GPU passthrough**: Pass a secondary GPU while keeping the primary for the host
- **Looking Glass integration**: Near-zero latency display for multi-GPU setups with auto-launch support
- **PCI passthrough screen**: Select PCI devices (GPUs, USB controllers, NVMe) for VM passthrough
- **System setup wizard**: One-click VFIO/IOMMU configuration with initramfs regeneration

**3D Graphics Acceleration**
- Para-virtualized 3D acceleration with `virtio-vga-gl` and SDL `gl=on`
- Tested on NVIDIA RTX-4090 with driver 590.48.01+
- Automatic SDL display selection for 3D-enabled VMs

**Snapshot Management**
- Create, restore, and delete snapshots for qcow2 disk images
- Visual snapshot list with timestamps and sizes
- Background operations with progress feedback

**Physical Disk Passthrough**
- Pass whole physical disks (NVMe, SATA/HDD, USB) through to guests as raw virtio-blk devices — at creation time (wizard "Physical Disk" mode) or on existing VMs (the "Passthrough Disks" management screen)
- Safety-filtered disk picker: the host system disk, mounted disks, swap members, and LVM/LUKS/RAID members are excluded with the reason shown; selection requires an explicit typed confirmation
- Stable `/dev/disk/by-id` device paths, per-disk firmware boot index, and launch-time preflight checks (device present, read/write access with fix hints, refuses mounted partitions)

**Network Configuration**
- **Multiple network adapters per VM**: add or remove NICs from a per-VM list (`[a]` add, `[d]` delete) in both the creation wizard and the Network Settings screen, each with its own model, backend, and settings
- Network backend selection per adapter: user/SLIRP (NAT), passt, bridge, or none
- Port forwarding with presets for common services (SSH, RDP, HTTP, HTTPS, VNC)
- Bridge networking with automatic bridge detection, status checklist, and setup guidance — cycling through bridge options also steps through every managed Virtual Network Manager network
- Configurable network adapter model and MAC address per NIC
- Network Settings has explicit Save/Discard with an unsaved-changes confirmation, at both the per-NIC and whole-screen level
- **Virtual Network Manager** (`n` on the main menu): create, edit, start/stop, and delete managed NAT or Isolated (host-only) networks with configurable subnets and built-in DHCP — ideal for multi-VM lab topologies. Host changes run via inspectable generated `net-up.sh`/`net-down.sh` scripts with explicit sudo

**Shared Folders**
- Share host directories with VMs using virtio-9p
- Add, remove, and edit shared folders from the management menu
- Automatic mount tag generation

**Clipboard Sharing (SPICE)**
- Bidirectional host ⇄ guest copy/paste when the display backend is `spice-app`
- The SPICE guest-agent channel is added to `launch.sh` automatically — no extra configuration
- Requires `virt-viewer`/`remote-viewer` on the host and `spice-vdagent` running in the guest:
  - Debian/Ubuntu/Kali: `sudo apt install spice-vdagent`
  - Fedora/RHEL: `sudo dnf install spice-vdagent`
  - Arch: `sudo pacman -S spice-vdagent`
  - Then enable the service (`sudo systemctl enable --now spice-vdagentd`) and reboot the guest

**USB Passthrough**
- USB device enumeration via libudev with sysfs fallback
- xHCI USB 3.0 controller with 8 ports (supports up to 8 USB 2.0 + 8 USB 3.0 devices)
- Persistent passthrough configuration
- Optional per-device firmware boot index (`b`) — passed-through USB drives can be the boot device
- Hub filtering and keyboard/mouse detection for passthrough validation

**VM Notes**
- Free-form personal notes for any VM from the management menu
- Multi-line text editor with full keyboard navigation
- Notes displayed in the main info panel and preserved across VM renames

**Launch Script Editor**
- Edit `launch.sh` scripts directly in the TUI
- Syntax-aware display with line numbers and horizontal scrolling
- Automatic QEMU configuration re-parsing after saves
- Automatic single-GPU passthrough script regeneration when applicable

**Additional Features**
- Vim-style navigation (j/k, arrows, mouse) with full clickable interface
- Multiple boot modes (normal, install, custom ISO, recovery image, floppy)
- Dynamic display backend detection per emulator (GTK, SDL, SPICE-app, VNC)
- Headless VM support (display=none) with process monitoring
- Stop/force-stop VMs (ACPI poweroff or SIGKILL)
- VM rename with persistent custom display names
- OS metadata system (publisher, release date, descriptions, fun facts) with user overrides in `~/.config/vm-curator/metadata/`
- 42+ ASCII art logos for classic and modern operating systems
- BTRFS copy-on-write auto-disable for VM directories
- First-time setup wizard for configuring the VM library directory
- Configurable settings with persistence

### Screenshots

```
 VM Curator (QEMU VM Library in ~/vm-space)
┌─────────────────────────────────────────────────────────────────────┐
│ ┌─────────────────────────┐  ┌────────────────────────────────────┐ │
│ │ VMs (35)                │  │       _    _ _           _        │ │
│ │ ──────────────────────  │  │      | |  | (_)         | |       │ │
│ │ 📁 Daily Drivers        │  │      | |/\| |_ _ __   __| | ___   │ │
│ │     > Windows 11    [*] │  │       \/  \/|_|_| |_|\__,_|\___/  │ │
│ │     > Debian 12         │  │                                   │ │
│ │ 📁 Retro                │  │   Windows 11                      │ │
│ │     > MS-DOS 6.22       │  │   Windows 11 | Microsoft | 2021   │ │
│ │     > Windows 95        │  │                                   │ │
│ │ 🐧 Ungrouped            │  │   Hardware                        │ │
│ │     > Ubuntu 24.04      │  │   Architecture: x86_64            │ │
│ │                         │  │   CPU: 4 cores (host)             │ │
│ │                         │  │   Memory: 4096 MB (4.0 GB)        │ │
│ │                         │  │   Features: KVM, UEFI, TPM        │ │
│ │                         │  │                                   │ │
│ │                         │  │   Disks                           │ │
│ │                         │  │     disk.qcow2 (25.5G/64.0G)      │ │
│ │                         │  │                                   │ │
│ │                         │  │   Network                         │ │
│ │                         │  │     NIC 1: virtio — bridge: vmc0  │ │
│ └─────────────────────────┘  └────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────────┤
│ [Enter] Launch  [m] Manage  [c] Create  [n] Networks  [g] Groups   │
└─────────────────────────────────────────────────────────────────────┘
```

### Installation

**AUR (Arch / Arch-derived)**

```bash
# Using your preferred AUR helper
paru -S vm-curator
yay -S vm-curator
```

**Homebrew (Linux, incl. atomic distros like Bluefin/Silverblue)**

```bash
brew install mroboff/tap/vm-curator
```

Installs the prebuilt x86_64 binary from [the tap](https://github.com/mroboff/homebrew-tap); QEMU itself is not bundled (see `brew info vm-curator`).

**crates.io**

```bash
cargo install vm-curator
```

**Binary Packages**

Pre-built packages (DEB, RPM, AppImage, tarball) are available from [GitHub Releases](https://github.com/mroboff/vm-curator/releases).

**Nix / NixOS**

```bash
# Run directly without installing
nix run github:mroboff/vm-curator

# Build the package
nix build .#default
```

For NixOS, add to `/etc/nixos/configuration.nix`:
```nix
{ pkgs, ... }:
{
  environment.systemPackages = [ pkgs.vm-curator ];
}
```

**From Source**

```bash
git clone https://github.com/mroboff/vm-curator.git
cd vm-curator
cargo build --release
```

The binary will be at `target/release/vm-curator`.

**Prerequisites**
- **Required**: QEMU (`qemu-system-*` binaries), qemu-img (for disk creation and snapshots), libudev
- **Build**: a recent Rust stable toolchain (see `rust-toolchain.toml`), libudev-dev (Debian/Ubuntu) or systemd-libs (Arch/Fedora)
- **Optional**:
  - OVMF/edk2 — UEFI boot support (`edk2-ovmf` on Arch, `ovmf` on Debian/Ubuntu)
  - swtpm — TPM 2.0 emulation (required for Windows 11 VMs)
  - virt-viewer — SPICE-app display backend
  - passt — passt network backend
  - dnsmasq — DHCP for managed virtual networks (nftables or iptables for NAT/isolation)
  - Looking Glass client — multi-GPU passthrough display
  - polkit — bridge networking permissions

### Usage

#### TUI Mode (default)

```bash
vm-curator
```

#### CLI Commands

```bash
# List all VMs
vm-curator list

# Launch a VM
vm-curator launch windows-95
vm-curator launch windows-95 --install    # Boot in install mode
vm-curator launch windows-95 --cdrom /path/to/image.iso

# View VM configuration
vm-curator info windows-95

# Import a VM
vm-curator  # then press 'i' for the import wizard

# Manage snapshots
vm-curator snapshot windows-95 list
vm-curator snapshot windows-95 create my-snapshot
vm-curator snapshot windows-95 restore my-snapshot
vm-curator snapshot windows-95 delete my-snapshot

# List available QEMU emulators
vm-curator emulators
```

### Key Bindings

#### Main Menu

| Key | Action |
|-----|--------|
| `j/k` or `Down/Up` | Navigate VM list |
| `Enter` | Launch selected VM |
| `m` | Open management menu |
| `x` | Stop VM (if running) |
| `c` | Open VM creation wizard |
| `i` | Open VM import wizard |
| `s` | Open settings |
| `n` | Virtual Network Manager |
| `g` | Manage VM Groups |
| `/` | Search/filter VMs |
| `?` | Show help |
| `PgUp/PgDn` | Scroll info panel |
| `Esc` | Back / Cancel |
| `q` | Quit |

#### VM Management

| Key | Action |
|-----|--------|
| `j/k` or `Down/Up` | Navigate menu |
| `Enter` | Select menu option |
| `1`–`9` | Jump directly to a menu option |
| `Esc` | Back |

Management menu options:
- Boot Options (normal, install, custom ISO)
- Snapshots
- USB Passthrough
- PCI Passthrough
- Passthrough Disks (whole physical disks)
- Shared Folders
- Network Settings
- Multi-GPU Passthrough (if enabled)
- Single GPU Passthrough (if enabled)
- Change Display
- 3D Acceleration toggle
- Edit Notes
- Rename VM
- Stop VM
- Reset VM (recreate disk)
- Delete VM
- Edit Raw Configuration

#### Create Wizard

| Key | Action |
|-----|--------|
| `j/k` or `Down/Up` | Navigate fields |
| `←/→` | Adjust / toggle the focused field |
| `Tab` | Edit the focused value as text (where supported) |
| `Enter` | Continue to next step |
| `Esc` | Previous step / cancel |

### Configuration

Settings are stored in `~/.config/vm-curator/config.toml` and can be edited via the Settings screen (`s` key).

```toml
# VM library location
vm_library_path = "~/vm-space"

# Default values for new VMs
default_memory_mb = 4096
default_cpu_cores = 2
default_disk_size_gb = 64
default_display = "gtk"      # gtk, sdl, spice-app, vnc
default_enable_kvm = true
# default_iso_path = "/path/to/isos"   # Directory the ISO browser opens in (omit = home)
# default_window_size = "1440x900"     # Initial VM window size (omit = VM default)

# Behavior
confirm_before_launch = true

# Multi-GPU passthrough (Looking Glass)
enable_multi_gpu_passthrough = false
default_ivshmem_size_mb = 64
show_gpu_warnings = true
looking_glass_client_path = ""       # Path to Looking Glass client
looking_glass_auto_launch = true     # Auto-launch client when VM starts

# Single GPU passthrough
single_gpu_enabled = false
single_gpu_auto_tty = false          # Experimental: auto switch TTY
single_gpu_dm_override = ""          # Override display manager detection
```

### VM Library Structure

VMs are expected in your library directory (default `~/vm-space/`) with this structure:

```
~/vm-space/
├── windows-95/
│   ├── launch.sh      # QEMU launch script (required)
│   └── disk.qcow2     # Disk image (qcow2 recommended for snapshots)
├── linux-debian/
│   ├── launch.sh
│   ├── disk.qcow2
│   └── install.iso    # Optional: installation media
└── macos-tiger/
    ├── launch.sh
    └── disk.qcow2
```

The `launch.sh` script should invoke QEMU. VM Curator parses this script to extract configuration and can generate new scripts via the creation wizard.

### OS Profiles

The creation wizard includes 130+ pre-configured profiles organized into 16 OS families:

**Microsoft**: DOS, Windows 1.x–3.x, Windows 95/98/ME, Windows NT/2000/XP/Vista, Windows 7/8/10/11, Server editions

**Apple**: Classic Mac OS (System 6–9), Mac OS X PowerPC (Cheetah–Tiger), Mac OS X Intel (Leopard–El Capitan), macOS (Sierra–Tahoe)

**Linux**: Arch, Manjaro, EndeavourOS, Garuda, CachyOS, Debian, Ubuntu, Mint, Pop!_OS, Fedora, RHEL, Rocky, Alma, Bazzite, openSUSE, Slackware, Gentoo, Void, NixOS, Alpine, and more

**BSD**: FreeBSD, GhostBSD, OpenBSD, NetBSD, DragonFly BSD

**Unix**: Solaris, OpenIndiana, illumos, HP-UX, IRIX, MINIX, QNX

**IBM**: OS/2, eComStation, ArcaOS, AIX

**Commodore**: AmigaOS, AROS, MorphOS

**Be / Haiku**: BeOS, Haiku

**NeXT**: NeXTSTEP, OpenStep

**Research**: Plan 9, 9front, Inferno

**Alternative**: SerenityOS, Redox, TempleOS, KolibriOS, MenuetOS, ReactOS

**Retro**: Atari TOS, CP/M, FreeDOS, DR-DOS, GEOS, RISC OS

**Mobile**: Android-x86, LineageOS, Bliss OS

**Infrastructure**: pfSense, OPNsense, OpenWrt, TrueNAS, Proxmox, ESXi

**Utilities**: GParted, Clonezilla, Memtest86+

**Other**: Catch-all for uncategorized VMs

Each profile includes optimal QEMU settings for that OS (emulator, machine type, CPU model, VGA, audio, network, disk interface, and more).

### Metadata Customization

**OS Information**: Override or add OS metadata in `~/.config/vm-curator/metadata/`:

```toml
# ~/.config/vm-curator/metadata/my-os.toml
[my-custom-os]
name = "My Custom OS"
publisher = "My Company"
release_date = "2024-01-01"
architecture = "x86_64"

[my-custom-os.blurb]
short = "A brief description"
long = "A longer description with history and details."

[my-custom-os.fun_facts]
facts = ["Fact 1", "Fact 2"]
```

**ASCII Art**: Add custom ASCII art in `~/.config/vm-curator/ascii/`.

**QEMU Profiles**: Override profiles in `~/.config/vm-curator/qemu_profiles.toml`.

### Dependencies

- **Runtime**: QEMU, qemu-img, libudev
- **Build**: a recent Rust stable toolchain (see `rust-toolchain.toml`), libudev-dev (Debian/Ubuntu) or systemd-libs (Arch)
- **Optional**: OVMF/edk2 (UEFI), swtpm (Windows 11 TPM), virt-viewer (SPICE-app), passt (networking), dnsmasq + nftables/iptables (managed virtual networks), Looking Glass client (multi-GPU), polkit (bridge networking)

### Cross-Distribution Compatibility

VM Curator automatically detects OVMF/UEFI firmware as matched CODE+VARS pairs (preferring 4M builds, including Fedora's qcow2-format firmware) across Linux distributions:
- Arch Linux: `/usr/share/edk2/x64/OVMF_CODE.4m.fd`
- Debian/Ubuntu: `/usr/share/OVMF/OVMF_CODE_4M.fd`
- Fedora/RHEL: `/usr/share/edk2/ovmf/OVMF_CODE_4M.qcow2`
- NixOS: Multiple search paths supported
- And more...

---

### Contributing

Contributions are welcome! If you find a bug or have an idea for an improvement, feel free to open an issue or submit a Pull Request.

**Help Wanted: ASCII Art**
As a TUI application, `vm-curator` relies on visual flair to stand out. I am specifically looking for help with:
* **Logo/Banner Art:** A cool ASCII banner for the startup screen.
* **Iconography:** Small, recognizable ASCII/block character icons for the TUI menus (e.g., stylized hard drives, network cards, or GPU icons).

If you have a knack for terminal aesthetics, your PRs are highly appreciated!

### Support & Maintenance Status

**`vm-curator`** was built to solve a specific, painful problem: getting high-performance, 3D-accelerated Linux VMs (via QEMU) without the overhead and complexity of `libvirt` or `virt-manager`.

This is a **personal passion project** that I am sharing with the community. While I use this tool daily and will fix critical bugs as I encounter them, please note:

* **Development Pace:** This project is maintained in my spare time. Feature requests will be considered but are not guaranteed.
* **The "As-Is" Philosophy:** The goal is a lean, transparent TUI. I prioritize stability and performance over comprehensive enterprise feature parity.

**If this tool saved you time or helped you get 3D Acceleration working without having to resort to passthrough:**

If you'd like to say thanks, you can support the project below. **Donations are a "thank you" for existing work, not a payment for future support.**

* **[GitHub Sponsors](https://github.com/sponsors/mroboff):** Best for one-time contributions (Goes to the RTX-Pro 6000 fund!)
* **[Ko-fi](https://ko-fi.com/mroboff):** Buy me a coffee (or a generic energy drink).

---

### License

MIT
