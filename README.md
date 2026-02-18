# Resguard - System Load Monitor with Auto-Throttling

<div align="center">
  
![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Linux](https://img.shields.io/badge/Linux-FCC624?style=for-the-badge&logo=linux&logoColor=black)
![cgroups](https://img.shields.io/badge/cgroups-v2-blue?style=for-the-badge)

**A production-ready system daemon that automatically detects and throttles resource-hungry processes using cgroups v2**

</div>

## 📋 Table of Contents
- [Features](#-features)
- [Architecture](#-architecture)
- [Quick Start](#-quick-start)
- [Configuration](#-configuration)
- [Stress Testing](#-stress-testing)
- [Common Issues](#-common-issues)
- [Project Structure](#-project-structure)
- [Complete Code](#-complete-code)

## 🚀 Features

- **Real-time monitoring** of CPU, memory, and system load
- **Rule-based auto-throttling** with configurable thresholds and durations
- **cgroups v2 integration** for CPU and memory limits
- **Process whitelist/blacklist** support
- **Systemd service detection** for service-level limiting
- **Live process viewer** showing top CPU consumers
- **Comprehensive action logging** for audit trails

┌─────────────────────────────────────┐
│ Resguard Daemon │
├─────────────────────────────────────┤
│ ┌─────────┐ ┌─────────┐ ┌─────┐ │
│ │ Metrics │ │ Process │ │Rules│ │
│ │ Scanner │──│ Scanner │──│Engine│ │
│ └─────────┘ └─────────┘ └──┬──┘ │
│ │ │ │ │
│ ▼ ▼ ▼ │
│ ┌───────────────────────────────┐ │
│ │ Cgroup Limiter │ │
│ │ (CPU/Memory/Stop actions) │ │
│ └───────────────────────────────┘ │
└─────────────────────────────────────┘
│
▼
┌─────────────────┐
│ Systemd/Init │
│ cgroups v2 │
└─────────────────┘







## ⚡ Quick Start

```bash
# Clone and build
git clone https://github.com/NickIBrody/Loadmonitor.git
cd Loadmonitor
cargo build --release

# Run (use sudo for cgroups)
sudo ./target/release/resguard


# Configuration
Create config.toml in the project root:
[general]
interval_secs = 5          # Check every 5 seconds
history_size = 1000        # Keep last 1000 actions

[limits]
cgroup_base_path = "/sys/fs/cgroup"
default_cpu_quota = 50.0   # Default CPU limit if not specified
default_memory_limit = 1073741824  # 1GB default
blacklist = ["systemd", "kernel", "init"]  # Never throttle these
whitelist = []              # Only throttle these (empty = all)

[[rules]]  # First rule: High CPU
name = "high-cpu"
duration_secs = 30           # Must exceed threshold for 30 seconds

[rules.condition]
type = "CpuOver"
threshold = 90.0             # CPU usage > 90%

[rules.action]
type = "LimitCpu"
max_percent = 40.0            # Throttle to 40% CPU

[[rules]]  # Second rule: High memory
name = "high-memory"
duration_secs = 60

[rules.condition]
type = "MemoryOver"
threshold = 8589934592       # 8GB

[rules.action]
type = "LimitMemory"
max_bytes = 4294967296        # Limit to 4GB


# Stress Testing

Terminal 1 - Run Resguard
cd ~/Loadmonitor
sudo cargo run


Terminal 2 - Generate Load
# Install stress tool if needed
sudo apt install stress -y

# CPU stress test (4 cores for 2 minutes)
stress --cpu 4 --timeout 120

# Memory stress test (2 processes allocating 1GB each)
stress --vm 2 --vm-bytes 1G --timeout 60

# Combined stress test
stress --cpu 2 --vm 1 --vm-bytes 512M --timeout 90


# Expected Output
✅ Resguard started. Monitoring processes...
📊 Check interval: 5 sec
⚙️  Rules loaded: 2

--- 14:23:45 ---
Total CPU: 34.0%
Memory: 2111 MB / 3795 MB

🔥 Top CPU processes:
  PID  34706: resguard             - CPU:  16.7%   RAM: 21 MB
  PID  34912: stress               - CPU:  12.5%   RAM: 0 MB
  PID  34913: stress               - CPU:   8.3%   RAM: 0 MB

⚠️  APPLYING LimitCpu(40.0) to PID 34912 (stress) - CPU: 22.0%
✅ Limit applied


# Key technical errors:

2. **Missing traits** - Forgot `SystemExt`, `CpuExt`, `PidExt` imports for system methods

3. **TOML structure** - `blacklist/whitelist` placed at root instead of inside `[limits]` section

4. **Type mismatches** - Confused `&Path` with `Option`, needed type casting for memory thresholds


# Project Structure
resguard/
├── Cargo.toml              # Dependencies and package config
├── config.toml             # User configuration
├── README.md               # This file
└── src/
    ├── main.rs             # Main daemon loop
    ├── config.rs            # Configuration parsing
    ├── errors.rs            # Error types
    ├── metrics/
    │   └── mod.rs           # CPU/memory metrics collection
    ├── process/
    │   └── mod.rs           # Process scanning and matching
    ├── rules/
    │   └── mod.rs           # Rule engine and evaluation
    └── limiter/
        └── mod.rs           # cgroups v2 integration



# License
MIT



## 🏗️ Architecture
