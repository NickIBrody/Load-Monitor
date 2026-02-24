# Load-Monitor - System Load Monitor with Auto-Throttling

  
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Status](https://img.shields.io/badge/status-alpha-orange.svg)
![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)
![Platform](https://img.shields.io/badge/platform-linux%20%7C%20wsl-lightgrey.svg)

# Load Monitor (Resguard)

A Linux process load monitoring and automatic resource limiting tool written in Rust.

Load Monitor continuously observes system processes and applies configurable rules to limit CPU or memory usage using cgroups v2 when defined thresholds are exceeded.

> Designed for Linux systems with cgroups v2 enabled.

---

## 🚀 Features

- Real-time system metrics collection (CPU, memory, load average)
- Per-process monitoring
- Rule-based resource limiting
- CPU and memory restriction via cgroups v2
- Configurable duration thresholds
- Process termination support
- Live top CPU process viewer
- Structured configuration via TOML

---

## 📦 Requirements

- Linux (cgroups v2 enabled)
- Rust (stable toolchain)
- Root privileges (required for cgroups manipulation)

Check cgroups version:

stat -fc %T /sys/fs/cgroup

It should return cgroup2fs.


# 🔧 Installation

Clone and build:

git clone https://github.com/yourusername/Load-Monitor.git
cd Load-Monitor
cargo build --release

Run (requires root):

sudo ./target/release/load-monitor


# ⚙️ Configuration

Create a config.toml in the project root.
Example Configuration
[general]
interval_secs = 5
history_size = 1000

[limits]
cgroup_base_path = "/sys/fs/cgroup"
default_cpu_quota = 50.0
default_memory_limit = 1073741824
blacklist = ["systemd", "kernel", "init"]
whitelist = []

[[rules]]
name = "high-cpu"
duration_secs = 30

[rules.condition]
type = "CpuOver"
threshold = 90.0

[rules.action]
type = "LimitCpu"
max_percent = 40.0

[[rules]]
name = "high-memory"
duration_secs = 60

[rules.condition]
type = "MemoryOver"
threshold = 8589934592

[rules.action]
type = "LimitMemory"
max_bytes = 4294967296


# How It Works

1 The application collects system metrics at a configurable interval.

2 It scans running processes.

3 Each process is evaluated against defined rules.

4 If a rule condition is continuously satisfied for duration_secs,
the configured action is applied.

5 The action is executed via Linux cgroups v2.

# Supported Rule Conditions

| Condition    | Description                                                  |
| ------------ | ------------------------------------------------------------ |
| `CpuOver`    | Triggers when process CPU usage exceeds threshold (%)        |
| `MemoryOver` | Triggers when process memory usage exceeds threshold (bytes) |
| `And`        | Combines multiple conditions                                 |



# Supported Actions
| Action        | Description                           |
| ------------- | ------------------------------------- |
| `LimitCpu`    | Applies CPU quota via `cpu.max`       |
| `LimitMemory` | Applies memory limit via `memory.max` |
| `Stop`        | Sends SIGTERM to the process          |


# 🔬 Stress Testing
Install stress tool:
sudo apt install stress
# CPU test:
stress --cpu 4 --timeout 120
# Memory test:
stress --vm 2 --vm-bytes 1G --timeout 60
You should see rules being triggered in the console output.


# ⚠️ Important Notes
Requires root privileges.

Works only on Linux with cgroups v2.

Improper rule configuration may limit critical system processes.

Cgroup directories are created dynamically per limited process.

Designed for controlled environments and testing.




# Limitations

No automatic cleanup of created cgroups (yet).

No persistent action history.

No remote management interface.

Not yet optimized for extremely high process counts.

Whitelist/blacklist matching may require further refinement.


# Development
Run in debug mode:
cargo run

Format code:
cargo fmt

Run lints:
cargo clippy


## ⚠️ Early Alpha / Experimental

This project is in early development and **not ready for production use**. 
Known issues: PID reuse risks, cgroup leaks, no graceful shutdown. 
Intended for learning, local testing, and community feedback. 
Use only in isolated environments and avoid long-running sessions. 
Contributions and constructive feedback welcome! 


# License & Disclaimer
Distributed under the MIT License. See LICENSE for details.
Disclaimer: This software is provided "as is", without warranty of any kind. Use at your own risk. The authors are not liable for any damage, data loss, or service disruption resulting from its use. Always test thoroughly in a safe environment before deploying anywhere that matters.


# 💡 Future Improvements

Persistent action logging

Automatic cgroup cleanup

Better PID reuse handling

Web dashboard

Structured logging

Prometheus metrics export

Unit and integration tests

# Author

Created as a Rust systems programming project focused on process control and resource management.

![GitHub stars](https://img.shields.io/github/stars/NickIBrody/Load-Monitor?style=for-the-badge&logo=github)
![GitHub forks](https://img.shields.io/github/forks/NickIBrody/Load-Monitor?style=for-the-badge&logo=github)
![GitHub issues](https://img.shields.io/github/issues/NickIBrody/Load-Monitor?style=for-the-badge&logo=github)

### 📂 Project Structure

```text
Load-Monitor/
├── Cargo.toml          # Dependencies & metadata
├── config.toml         # Example configuration
├── README.md           # Project documentation
├── LICENSE             # MIT License
└── src/
    ├── main.rs         # Entry point & async runtime
    ├── config.rs       # TOML config parsing (serde)
    ├── errors.rs       # Custom error types (thiserror)
    ├── metrics/
    │   └── mod.rs      # System metrics collection (sysinfo)
    ├── process/
    │   └── mod.rs      # Process scanning & filtering
    ├── rules/
    │   └── mod.rs      # Rule engine & condition evaluation
    └── limiter/
        └── mod.rs      # Cgroups v2 resource limiting


