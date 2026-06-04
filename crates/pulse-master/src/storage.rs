use anyhow::Result;
use pulse_proto::pb::{MetricsPayload, RegisterRequest};
use sqlx::PgPool;
use sqlx::types::chrono::{DateTime, Utc};

#[derive(Clone)]
pub struct Storage {
    pool: PgPool,
}

impl Storage {
    pub async fn connect(url: &str) -> Result<Self> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(20)
            .acquire_timeout(std::time::Duration::from_secs(5))
            .connect(url)
            .await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn migrate(&self) -> Result<()> {
        let sql = include_str!("../../../migrations/001_init.sql");
        sqlx::raw_sql(sql).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn upsert_agent(&self, id: &str, req: &RegisterRequest) -> Result<()> {
        sqlx::query(
            "INSERT INTO agents (id, hostname, os, arch, agent_version, labels)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (id) DO UPDATE SET
                hostname = EXCLUDED.hostname,
                agent_version = EXCLUDED.agent_version,
                last_seen_at = NOW()",
        )
        .bind(id)
        .bind(&req.hostname)
        .bind(&req.os)
        .bind(&req.arch)
        .bind(&req.agent_version)
        .bind(&req.labels)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_metrics(&self, payload: &MetricsPayload) -> Result<()> {
        let time = DateTime::<Utc>::from_timestamp_millis(payload.timestamp_ms as i64).unwrap_or_else(Utc::now);

        let (cpu_total, cpu_cores, load1, load5, load15) = match &payload.cpu {
            Some(cpu) => (
                Some(cpu.usage_total_bp as i32),
                Some(cpu.usage_per_core_bp.iter().map(|v| *v as i32).collect::<Vec<_>>()),
                Some(cpu.load_avg_1m_cp as i32),
                Some(cpu.load_avg_5m_cp as i32),
                Some(cpu.load_avg_15m_cp as i32),
            ),
            None => (None, None, None, None, None),
        };

        let (mem_total, mem_used, mem_avail, swap_total, swap_used, cg_limit, cg_usage) = match &payload.memory {
            Some(m) => (
                Some(m.total_bytes as i64),
                Some(m.used_bytes as i64),
                Some(m.available_bytes as i64),
                Some(m.swap_total_bytes as i64),
                Some(m.swap_used_bytes as i64),
                Some(m.cgroup_limit_bytes as i64),
                Some(m.cgroup_usage_bytes as i64),
            ),
            None => (None, None, None, None, None, None, None),
        };

        let (uptime, procs_total, procs_running) = match &payload.system {
            Some(s) => (Some(s.uptime_secs as i64), Some(s.processes_total as i32), Some(s.processes_running as i32)),
            None => (None, None, None),
        };

        sqlx::query(
            "INSERT INTO metrics (time, agent_id, cpu_total_bp, cpu_cores_bp, load_1m, load_5m, load_15m,
                mem_total, mem_used, mem_available, swap_total, swap_used, cgroup_limit, cgroup_usage,
                uptime_secs, procs_total, procs_running, battery_pct)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)",
        )
        .bind(time)
        .bind(&payload.agent_id)
        .bind(cpu_total)
        .bind(&cpu_cores)
        .bind(load1)
        .bind(load5)
        .bind(load15)
        .bind(mem_total)
        .bind(mem_used)
        .bind(mem_avail)
        .bind(swap_total)
        .bind(swap_used)
        .bind(cg_limit)
        .bind(cg_usage)
        .bind(uptime)
        .bind(procs_total)
        .bind(procs_running)
        .bind(payload.battery_pct as i32)
        .execute(&self.pool)
        .await?;

        // Network metrics
        for net in &payload.net {
            sqlx::query(
                "INSERT INTO net_metrics (time, agent_id, interface, rx_bytes, tx_bytes, rx_packets, tx_packets, rx_errors, tx_errors)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            )
            .bind(time)
            .bind(&payload.agent_id)
            .bind(&net.interface)
            .bind(net.rx_bytes_sec as i64)
            .bind(net.tx_bytes_sec as i64)
            .bind(net.rx_packets_sec as i64)
            .bind(net.tx_packets_sec as i64)
            .bind(net.rx_errors as i64)
            .bind(net.tx_errors as i64)
            .execute(&self.pool)
            .await?;
        }

        // Container metrics
        for ctr in &payload.containers {
            sqlx::query(
                "INSERT INTO container_metrics (time, agent_id, container_id, container_name, image, state, cpu_bp, mem_usage, mem_limit, net_rx_bytes, net_tx_bytes)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            )
            .bind(time)
            .bind(&payload.agent_id)
            .bind(&ctr.id)
            .bind(&ctr.name)
            .bind(&ctr.image)
            .bind(&ctr.state)
            .bind(ctr.cpu_bp as i32)
            .bind(ctr.mem_usage as i64)
            .bind(ctr.mem_limit as i64)
            .bind(ctr.net_rx_bytes as i64)
            .bind(ctr.net_tx_bytes as i64)
            .execute(&self.pool)
            .await?;
        }

        // Disk usage
        for disk in &payload.disks {
            sqlx::query(
                "INSERT INTO disk_metrics (time, agent_id, mount_point, total_bytes, used_bytes)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(time)
            .bind(&payload.agent_id)
            .bind(&disk.mount_point)
            .bind(disk.total_bytes as i64)
            .bind(disk.used_bytes as i64)
            .execute(&self.pool)
            .await?;
        }

        // Disk I/O
        for dio in &payload.disk_io {
            sqlx::query(
                "INSERT INTO disk_io_metrics (time, agent_id, device, read_bytes, write_bytes, read_ops, write_ops)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(time)
            .bind(&payload.agent_id)
            .bind(&dio.device)
            .bind(dio.read_bytes as i64)
            .bind(dio.write_bytes as i64)
            .bind(dio.read_ops as i64)
            .bind(dio.write_ops as i64)
            .execute(&self.pool)
            .await?;
        }

        // Disk health
        for dh in &payload.disk_health {
            sqlx::query(
                "INSERT INTO disk_health (time, agent_id, device, model, temp_c, power_on_hours, healthy)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(time)
            .bind(&payload.agent_id)
            .bind(&dh.device)
            .bind(&dh.model)
            .bind(dh.temp_c as i32)
            .bind(dh.power_on_hours as i32)
            .bind(dh.healthy)
            .execute(&self.pool)
            .await?;
        }

        // Temperatures
        for temp in &payload.temperatures {
            sqlx::query(
                "INSERT INTO temperature_metrics (time, agent_id, zone, temp_mc)
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(time)
            .bind(&payload.agent_id)
            .bind(&temp.zone)
            .bind(temp.temp_mc)
            .execute(&self.pool)
            .await?;
        }

        // GPUs
        for gpu in &payload.gpus {
            sqlx::query(
                "INSERT INTO gpu_metrics (time, agent_id, gpu_index, name, utilization_pct, mem_used, mem_total, temp_c, power_watts)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            )
            .bind(time)
            .bind(&payload.agent_id)
            .bind(gpu.index as i32)
            .bind(&gpu.name)
            .bind(gpu.utilization_pct as i32)
            .bind(gpu.mem_used as i64)
            .bind(gpu.mem_total as i64)
            .bind(gpu.temp_c)
            .bind(gpu.power_watts as i32)
            .execute(&self.pool)
            .await?;
        }

        // Top processes
        for proc in &payload.top_processes {
            sqlx::query(
                "INSERT INTO process_metrics (time, agent_id, pid, name, cpu_bp, rss_bytes)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(time)
            .bind(&payload.agent_id)
            .bind(proc.pid as i32)
            .bind(&proc.name)
            .bind(proc.cpu_bp as i32)
            .bind(proc.rss_bytes as i64)
            .execute(&self.pool)
            .await?;
        }

        // SSH sessions
        for sess in &payload.ssh_sessions {
            sqlx::query(
                "INSERT INTO ssh_sessions (time, agent_id, username, remote_ip, tty, pid)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(time)
            .bind(&payload.agent_id)
            .bind(&sess.user)
            .bind(&sess.remote_ip)
            .bind(&sess.tty)
            .bind(sess.pid as i32)
            .execute(&self.pool)
            .await?;
        }

        // Update last_seen
        sqlx::query("UPDATE agents SET last_seen_at = NOW() WHERE id = $1")
            .bind(&payload.agent_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_offline_agents(&self, timeout_secs: i64) -> Result<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT id FROM agents WHERE last_seen_at < NOW() - make_interval(secs => $1) AND COALESCE(status, 'online') = 'online'"
        )
        .bind(timeout_secs as f64)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    pub async fn set_agent_status(&self, agent_id: &str, status: &str) -> Result<()> {
        sqlx::query("UPDATE agents SET status = $2 WHERE id = $1")
            .bind(agent_id)
            .bind(status)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
