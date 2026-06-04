CREATE EXTENSION IF NOT EXISTS timescaledb;

-- Agent inventory
CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    hostname TEXT NOT NULL,
    os TEXT NOT NULL,
    arch TEXT NOT NULL,
    agent_version TEXT NOT NULL,
    labels TEXT[] NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'online',
    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Metrics hypertable
CREATE TABLE IF NOT EXISTS metrics (
    time TIMESTAMPTZ NOT NULL,
    agent_id TEXT NOT NULL,
    cpu_total_bp INTEGER,
    cpu_cores_bp INTEGER[],
    load_1m INTEGER,
    load_5m INTEGER,
    load_15m INTEGER,
    mem_total BIGINT,
    mem_used BIGINT,
    mem_available BIGINT,
    swap_total BIGINT,
    swap_used BIGINT,
    cgroup_limit BIGINT,
    cgroup_usage BIGINT,
    uptime_secs BIGINT,
    procs_total INTEGER,
    procs_running INTEGER,
    battery_pct INTEGER
);

SELECT create_hypertable('metrics', by_range('time'), if_not_exists => TRUE);

-- Schema evolution: add columns that may not exist in older installations
ALTER TABLE metrics ADD COLUMN IF NOT EXISTS battery_pct INTEGER;

-- Network metrics (separate hypertable — different cardinality)
CREATE TABLE IF NOT EXISTS net_metrics (
    time TIMESTAMPTZ NOT NULL,
    agent_id TEXT NOT NULL,
    interface TEXT NOT NULL,
    rx_bytes BIGINT,
    tx_bytes BIGINT,
    rx_packets BIGINT,
    tx_packets BIGINT,
    rx_errors BIGINT,
    tx_errors BIGINT
);

SELECT create_hypertable('net_metrics', by_range('time'), if_not_exists => TRUE);

-- Compression: compress chunks older than 7 days
ALTER TABLE metrics SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'agent_id'
);

ALTER TABLE net_metrics SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'agent_id,interface'
);

DO $$ BEGIN
  PERFORM add_compression_policy('metrics', INTERVAL '7 days');
EXCEPTION WHEN others THEN NULL;
END $$;

DO $$ BEGIN
  PERFORM add_compression_policy('net_metrics', INTERVAL '7 days');
EXCEPTION WHEN others THEN NULL;
END $$;

-- Retention: drop data older than 90 days
DO $$ BEGIN
  PERFORM add_retention_policy('metrics', INTERVAL '90 days');
EXCEPTION WHEN others THEN NULL;
END $$;

DO $$ BEGIN
  PERFORM add_retention_policy('net_metrics', INTERVAL '90 days');
EXCEPTION WHEN others THEN NULL;
END $$;

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_metrics_agent_time ON metrics (agent_id, time DESC);
CREATE INDEX IF NOT EXISTS idx_net_agent_time ON net_metrics (agent_id, time DESC);

-- Container metrics
CREATE TABLE IF NOT EXISTS container_metrics (
    time TIMESTAMPTZ NOT NULL,
    agent_id TEXT NOT NULL,
    container_id TEXT NOT NULL,
    container_name TEXT NOT NULL,
    image TEXT,
    state TEXT,
    cpu_bp INTEGER,
    mem_usage BIGINT,
    mem_limit BIGINT,
    net_rx_bytes BIGINT,
    net_tx_bytes BIGINT
);

SELECT create_hypertable('container_metrics', by_range('time'), if_not_exists => TRUE);

ALTER TABLE container_metrics SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'agent_id,container_id'
);

DO $$ BEGIN
  PERFORM add_compression_policy('container_metrics', INTERVAL '7 days');
EXCEPTION WHEN others THEN NULL;
END $$;

DO $$ BEGIN
  PERFORM add_retention_policy('container_metrics', INTERVAL '90 days');
EXCEPTION WHEN others THEN NULL;
END $$;

CREATE INDEX IF NOT EXISTS idx_container_agent_time ON container_metrics (agent_id, time DESC);

-- Disk usage metrics
CREATE TABLE IF NOT EXISTS disk_metrics (
    time TIMESTAMPTZ NOT NULL,
    agent_id TEXT NOT NULL,
    mount_point TEXT NOT NULL,
    total_bytes BIGINT,
    used_bytes BIGINT
);

SELECT create_hypertable('disk_metrics', by_range('time'), if_not_exists => TRUE);

ALTER TABLE disk_metrics SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'agent_id,mount_point'
);

DO $$ BEGIN
  PERFORM add_compression_policy('disk_metrics', INTERVAL '7 days');
EXCEPTION WHEN others THEN NULL;
END $$;

DO $$ BEGIN
  PERFORM add_retention_policy('disk_metrics', INTERVAL '90 days');
EXCEPTION WHEN others THEN NULL;
END $$;

CREATE INDEX IF NOT EXISTS idx_disk_agent_time ON disk_metrics (agent_id, time DESC);

-- Disk I/O metrics
CREATE TABLE IF NOT EXISTS disk_io_metrics (
    time TIMESTAMPTZ NOT NULL,
    agent_id TEXT NOT NULL,
    device TEXT NOT NULL,
    read_bytes BIGINT,
    write_bytes BIGINT,
    read_ops BIGINT,
    write_ops BIGINT
);

SELECT create_hypertable('disk_io_metrics', by_range('time'), if_not_exists => TRUE);

ALTER TABLE disk_io_metrics SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'agent_id,device'
);

DO $$ BEGIN
  PERFORM add_compression_policy('disk_io_metrics', INTERVAL '7 days');
EXCEPTION WHEN others THEN NULL;
END $$;

DO $$ BEGIN
  PERFORM add_retention_policy('disk_io_metrics', INTERVAL '90 days');
EXCEPTION WHEN others THEN NULL;
END $$;

CREATE INDEX IF NOT EXISTS idx_disk_io_agent_time ON disk_io_metrics (agent_id, time DESC);

-- Disk health / SMART
CREATE TABLE IF NOT EXISTS disk_health (
    time TIMESTAMPTZ NOT NULL,
    agent_id TEXT NOT NULL,
    device TEXT NOT NULL,
    model TEXT,
    temp_c INTEGER,
    power_on_hours INTEGER,
    healthy BOOLEAN
);

SELECT create_hypertable('disk_health', by_range('time'), if_not_exists => TRUE);

ALTER TABLE disk_health SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'agent_id,device'
);

DO $$ BEGIN
  PERFORM add_compression_policy('disk_health', INTERVAL '7 days');
EXCEPTION WHEN others THEN NULL;
END $$;

DO $$ BEGIN
  PERFORM add_retention_policy('disk_health', INTERVAL '90 days');
EXCEPTION WHEN others THEN NULL;
END $$;

CREATE INDEX IF NOT EXISTS idx_disk_health_agent_time ON disk_health (agent_id, time DESC);

-- Temperature metrics
CREATE TABLE IF NOT EXISTS temperature_metrics (
    time TIMESTAMPTZ NOT NULL,
    agent_id TEXT NOT NULL,
    zone TEXT NOT NULL,
    temp_mc INTEGER
);

SELECT create_hypertable('temperature_metrics', by_range('time'), if_not_exists => TRUE);

ALTER TABLE temperature_metrics SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'agent_id,zone'
);

DO $$ BEGIN
  PERFORM add_compression_policy('temperature_metrics', INTERVAL '7 days');
EXCEPTION WHEN others THEN NULL;
END $$;

DO $$ BEGIN
  PERFORM add_retention_policy('temperature_metrics', INTERVAL '90 days');
EXCEPTION WHEN others THEN NULL;
END $$;

CREATE INDEX IF NOT EXISTS idx_temp_agent_time ON temperature_metrics (agent_id, time DESC);

-- GPU metrics
CREATE TABLE IF NOT EXISTS gpu_metrics (
    time TIMESTAMPTZ NOT NULL,
    agent_id TEXT NOT NULL,
    gpu_index INTEGER NOT NULL,
    name TEXT,
    utilization_pct INTEGER,
    mem_used BIGINT,
    mem_total BIGINT,
    temp_c INTEGER,
    power_watts INTEGER
);

SELECT create_hypertable('gpu_metrics', by_range('time'), if_not_exists => TRUE);

ALTER TABLE gpu_metrics SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'agent_id,gpu_index'
);

DO $$ BEGIN
  PERFORM add_compression_policy('gpu_metrics', INTERVAL '7 days');
EXCEPTION WHEN others THEN NULL;
END $$;

DO $$ BEGIN
  PERFORM add_retention_policy('gpu_metrics', INTERVAL '90 days');
EXCEPTION WHEN others THEN NULL;
END $$;

CREATE INDEX IF NOT EXISTS idx_gpu_agent_time ON gpu_metrics (agent_id, time DESC);

-- Top processes
CREATE TABLE IF NOT EXISTS process_metrics (
    time TIMESTAMPTZ NOT NULL,
    agent_id TEXT NOT NULL,
    pid INTEGER NOT NULL,
    name TEXT NOT NULL,
    cpu_bp INTEGER,
    rss_bytes BIGINT
);

SELECT create_hypertable('process_metrics', by_range('time'), if_not_exists => TRUE);

ALTER TABLE process_metrics SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'agent_id'
);

DO $$ BEGIN
  PERFORM add_compression_policy('process_metrics', INTERVAL '7 days');
EXCEPTION WHEN others THEN NULL;
END $$;

DO $$ BEGIN
  PERFORM add_retention_policy('process_metrics', INTERVAL '90 days');
EXCEPTION WHEN others THEN NULL;
END $$;

CREATE INDEX IF NOT EXISTS idx_process_agent_time ON process_metrics (agent_id, time DESC);

-- SSH sessions
CREATE TABLE IF NOT EXISTS ssh_sessions (
    time TIMESTAMPTZ NOT NULL,
    agent_id TEXT NOT NULL,
    username TEXT NOT NULL,
    remote_ip TEXT,
    tty TEXT,
    pid INTEGER
);

SELECT create_hypertable('ssh_sessions', by_range('time'), if_not_exists => TRUE);

ALTER TABLE ssh_sessions SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'agent_id'
);

DO $$ BEGIN
  PERFORM add_compression_policy('ssh_sessions', INTERVAL '7 days');
EXCEPTION WHEN others THEN NULL;
END $$;

DO $$ BEGIN
  PERFORM add_retention_policy('ssh_sessions', INTERVAL '90 days');
EXCEPTION WHEN others THEN NULL;
END $$;

CREATE INDEX IF NOT EXISTS idx_ssh_agent_time ON ssh_sessions (agent_id, time DESC);

-- Continuous aggregates: pre-compute hourly rollups for dashboards
DO $$ BEGIN
  CREATE MATERIALIZED VIEW IF NOT EXISTS metrics_hourly
  WITH (timescaledb.continuous) AS
  SELECT time_bucket('1 hour', time) AS bucket,
         agent_id,
         avg(cpu_total_bp) AS avg_cpu_bp,
         max(cpu_total_bp) AS max_cpu_bp,
         avg(mem_used) AS avg_mem_used,
         max(mem_used) AS max_mem_used
  FROM metrics
  GROUP BY bucket, agent_id;
EXCEPTION WHEN others THEN NULL;
END $$;

DO $$ BEGIN
  PERFORM add_continuous_aggregate_policy('metrics_hourly',
    start_offset => INTERVAL '3 hours',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour');
EXCEPTION WHEN others THEN NULL;
END $$;
