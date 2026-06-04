-- Vixpi Pulse — SQLite Schema (embedded mode)
-- Runs on master boot when DATABASE_URL is absent.

PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
PRAGMA busy_timeout=5000;
PRAGMA cache_size=-8000; -- 8 MB cache

CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    hostname TEXT NOT NULL,
    os TEXT NOT NULL,
    arch TEXT NOT NULL,
    agent_version TEXT NOT NULL,
    labels TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'online',
    registered_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    last_seen_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS metrics (
    time TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    agent_id TEXT NOT NULL,
    cpu_total_bp INTEGER,
    cpu_cores_bp TEXT, -- JSON array
    load_1m INTEGER,
    load_5m INTEGER,
    load_15m INTEGER,
    mem_total INTEGER,
    mem_used INTEGER,
    mem_available INTEGER,
    swap_total INTEGER,
    swap_used INTEGER,
    cgroup_limit INTEGER,
    cgroup_usage INTEGER,
    uptime_secs INTEGER,
    procs_total INTEGER,
    procs_running INTEGER,
    battery_pct INTEGER
);
CREATE INDEX IF NOT EXISTS idx_metrics_agent_time ON metrics (agent_id, time DESC);

CREATE TABLE IF NOT EXISTS net_metrics (
    time TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    agent_id TEXT NOT NULL,
    interface TEXT NOT NULL,
    rx_bytes INTEGER,
    tx_bytes INTEGER,
    rx_packets INTEGER,
    tx_packets INTEGER,
    rx_errors INTEGER,
    tx_errors INTEGER
);
CREATE INDEX IF NOT EXISTS idx_net_agent_time ON net_metrics (agent_id, time DESC);

CREATE TABLE IF NOT EXISTS container_metrics (
    time TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    agent_id TEXT NOT NULL,
    container_id TEXT NOT NULL,
    container_name TEXT NOT NULL,
    image TEXT,
    state TEXT,
    cpu_bp INTEGER,
    mem_usage INTEGER,
    mem_limit INTEGER,
    net_rx_bytes INTEGER,
    net_tx_bytes INTEGER
);
CREATE INDEX IF NOT EXISTS idx_container_agent_time ON container_metrics (agent_id, time DESC);

CREATE TABLE IF NOT EXISTS disk_metrics (
    time TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    agent_id TEXT NOT NULL,
    mount_point TEXT NOT NULL,
    total_bytes INTEGER,
    used_bytes INTEGER
);
CREATE INDEX IF NOT EXISTS idx_disk_agent_time ON disk_metrics (agent_id, time DESC);

CREATE TABLE IF NOT EXISTS disk_io_metrics (
    time TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    agent_id TEXT NOT NULL,
    device TEXT NOT NULL,
    read_bytes INTEGER,
    write_bytes INTEGER,
    read_ops INTEGER,
    write_ops INTEGER
);
CREATE INDEX IF NOT EXISTS idx_disk_io_agent_time ON disk_io_metrics (agent_id, time DESC);

CREATE TABLE IF NOT EXISTS disk_health (
    time TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    agent_id TEXT NOT NULL,
    device TEXT NOT NULL,
    model TEXT,
    temp_c INTEGER,
    power_on_hours INTEGER,
    healthy INTEGER -- SQLite has no BOOLEAN, use 0/1
);
CREATE INDEX IF NOT EXISTS idx_disk_health_agent_time ON disk_health (agent_id, time DESC);

CREATE TABLE IF NOT EXISTS temperature_metrics (
    time TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    agent_id TEXT NOT NULL,
    zone TEXT NOT NULL,
    temp_mc INTEGER
);
CREATE INDEX IF NOT EXISTS idx_temp_agent_time ON temperature_metrics (agent_id, time DESC);

CREATE TABLE IF NOT EXISTS gpu_metrics (
    time TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    agent_id TEXT NOT NULL,
    gpu_index INTEGER NOT NULL,
    name TEXT,
    utilization_pct INTEGER,
    mem_used INTEGER,
    mem_total INTEGER,
    temp_c INTEGER,
    power_watts INTEGER
);
CREATE INDEX IF NOT EXISTS idx_gpu_agent_time ON gpu_metrics (agent_id, time DESC);

CREATE TABLE IF NOT EXISTS process_metrics (
    time TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    agent_id TEXT NOT NULL,
    pid INTEGER NOT NULL,
    name TEXT NOT NULL,
    cpu_bp INTEGER,
    rss_bytes INTEGER
);
CREATE INDEX IF NOT EXISTS idx_process_agent_time ON process_metrics (agent_id, time DESC);

CREATE TABLE IF NOT EXISTS ssh_sessions (
    time TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    agent_id TEXT NOT NULL,
    username TEXT NOT NULL,
    remote_ip TEXT,
    tty TEXT,
    pid INTEGER
);
CREATE INDEX IF NOT EXISTS idx_ssh_agent_time ON ssh_sessions (agent_id, time DESC);

CREATE TABLE IF NOT EXISTS connection_metrics (
    time TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    agent_id TEXT NOT NULL,
    port INTEGER NOT NULL,
    service TEXT NOT NULL,
    active_count INTEGER NOT NULL,
    remote_ips TEXT NOT NULL -- JSON array
);
CREATE INDEX IF NOT EXISTS idx_conn_agent_time ON connection_metrics (agent_id, time DESC);
