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
    procs_running INTEGER
);

SELECT create_hypertable('metrics', by_range('time'), if_not_exists => TRUE);

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
