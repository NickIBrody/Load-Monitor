# ⚡ Load Monitor (Resguard)

![Version](https://img.shields.io/badge/version-0.2.0-blue.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Status](https://img.shields.io/badge/status-beta-yellow.svg)
![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)
![Platform](https://img.shields.io/badge/platform-linux%20%7C%20cgroups%20v2-lightgrey.svg)
![GitHub stars](https://img.shields.io/github/stars/NickIBrody/Load-Monitor?style=flat&logo=github)

A Linux system load monitor with **interactive TUI**, real-time metrics, and automatic process throttling via cgroups v2 — written in Rust.

---

## ✨ Features

- **Interactive TUI** — full-screen terminal UI built with [ratatui](https://github.com/ratatui-org/ratatui)
- **Live gauges** — CPU%, RAM usage, load averages (1m/5m/15m) with color thresholds
- **Process table** — sortable by CPU, RAM, PID, or name with keyboard navigation
- **Rule engine** — configurable conditions + actions with duration threshold before triggering
- **Settings editor** — edit config fields live inside the TUI, save to disk with `[w]`
- **Action log** — full history of applied limits/kills, scrollable
- **cgroups v2** — CPU quota and memory limit enforcement via Linux control groups
- **Auto-cleanup** — created cgroup dirs are removed on graceful exit
- **Config hot-reload** — press `[r]` to reload `config.toml` without restarting

---

## 🎮 Keyboard Shortcuts

| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Switch tabs |
| `1` `2` `3` `4` | Jump to Monitor / Rules / Settings / Log |
| `↑` `↓` or `j` `k` | Navigate process list / settings fields |
| `s` | Cycle sort: CPU → RAM → PID → Name |
| `r` | Hot-reload `config.toml` |
| `Enter` or `e` | Edit selected settings field |
| `w` | Save settings to `config.toml` |
| `c` | Clear action log |
| `q` / `Ctrl+C` | Quit (cleans up cgroups) |

---

## 📦 Requirements

- Linux with **cgroups v2** enabled
- Rust stable toolchain (1.70+)
- Root privileges (required for cgroups manipulation)

Check cgroups version:
```bash
stat -fc %T /sys/fs/cgroup
# should print: cgroup2fs
```

---

## 🔧 Installation

```bash
git clone https://github.com/NickIBrody/Load-Monitor.git
cd Load-Monitor
cargo build --release
sudo ./target/release/load-monitor
```

Custom config path:
```bash
sudo ./target/release/load-monitor /path/to/config.toml
```

---

## ⚙️ Configuration

`config.toml` in the project root:

```toml
[general]
interval_secs = 5        # metrics collection interval
history_size  = 1000     # max entries in action log

[limits]
cgroup_base_path     = "/sys/fs/cgroup"
default_cpu_quota    = 50.0        # %
default_memory_limit = 1073741824  # bytes (1 GB)
blacklist = ["systemd", "kernel", "init"]
whitelist = []

[[rules]]
name          = "high-cpu"
duration_secs = 30        # must violate condition for this long before action fires

[rules.condition]
type      = "CpuOver"
threshold = 90.0          # %

[rules.action]
type        = "LimitCpu"
max_percent = 40.0

[[rules]]
name          = "high-memory"
duration_secs = 60

[rules.condition]
type      = "MemoryOver"
threshold = 8589934592    # bytes (8 GB)

[rules.action]
type      = "LimitMemory"
max_bytes = 4294967296    # bytes (4 GB)
```

### Supported Conditions

| Condition | Description |
|---|---|
| `CpuOver` | Process CPU usage exceeds `threshold` % |
| `MemoryOver` | Process memory exceeds `threshold` bytes |
| `And` | All sub-conditions must be true |

### Supported Actions

| Action | Description |
|---|---|
| `LimitCpu` | Apply CPU quota via `cpu.max` (cgroups v2) |
| `LimitMemory` | Apply memory cap via `memory.max` (cgroups v2) |
| `Stop` | Send SIGTERM to the process |

---

## 🔬 Stress Testing

```bash
sudo apt install stress

# Trigger the CPU rule
stress --cpu 4 --timeout 120

# Trigger the memory rule
stress --vm 2 --vm-bytes 5G --timeout 60
```

Watch the **Monitor** tab react and the **Log** tab fill with applied actions.

---

## 📂 Project Structure

```
Load-Monitor/
├── Cargo.toml          # dependencies & metadata
├── config.toml         # example configuration
├── src/
│   ├── main.rs         # entry point, terminal setup
│   ├── app.rs          # application state & event loop
│   ├── ui.rs           # TUI rendering (ratatui)
│   ├── config.rs       # TOML config parsing & saving
│   ├── errors.rs       # custom error types
│   ├── metrics/mod.rs  # system metrics (sysinfo)
│   ├── process/mod.rs  # process scanning & filtering
│   ├── rules/mod.rs    # rule engine & condition evaluation
│   └── limiter/mod.rs  # cgroups v2 resource limiting
└── screenshots/
```

---

## ⚠️ Notes

- Requires **root** — cgroup manipulation needs elevated privileges
- **Linux only** — cgroups v2 is a Linux-specific feature
- Improper rules may throttle critical system processes — use the blacklist
- Designed for controlled environments, testing, and learning

---

## 🔮 Roadmap

- [ ] Per-process CPU history sparkline in TUI
- [ ] Prometheus metrics export
- [ ] Web dashboard
- [ ] Persistent action log (SQLite)
- [ ] Unit & integration tests

---

## 👥 Authors

- **[NickIBrody](https://github.com/NickIBrody)** — project lead
- **[GuardionSpend](https://github.com/GuardionSpend)** — contributor

---

## 📄 License

MIT — see [LICENSE](LICENSE) for details.

> Use only in isolated/test environments. Authors are not liable for data loss or system disruption.
