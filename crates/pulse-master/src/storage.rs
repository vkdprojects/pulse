use anyhow::Result;
use pulse_proto::pb::{MetricsPayload, RegisterRequest};
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::{PgPool, SqlitePool};

#[derive(Clone)]
pub enum Storage {
    Postgres(PgPool),
    Sqlite(SqlitePool),
}

impl Storage {
    pub async fn connect(url: Option<&str>) -> Result<Self> {
        match url {
            Some(u) if u.starts_with("postgres://") || u.starts_with("postgresql://") => {
                let pool = sqlx::postgres::PgPoolOptions::new()
                    .max_connections(20)
                    .acquire_timeout(std::time::Duration::from_secs(5))
                    .connect(u)
                    .await?;
                Ok(Storage::Postgres(pool))
            }
            Some(path) => Self::connect_sqlite(path).await,
            None => {
                let default = if std::path::Path::new("/var/lib/pulse").exists()
                    || std::fs::create_dir_all("/var/lib/pulse").is_ok()
                {
                    "/var/lib/pulse/pulse.db"
                } else {
                    std::fs::create_dir_all("data").ok();
                    "data/pulse.db"
                };
                Self::connect_sqlite(default).await
            }
        }
    }

    async fn connect_sqlite(path: &str) -> Result<Self> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let url = format!("sqlite://{}?mode=rwc", path);
        let pool = sqlx::sqlite::SqlitePoolOptions::new().max_connections(5).connect(&url).await?;
        Ok(Storage::Sqlite(pool))
    }

    pub fn pg_pool(&self) -> Option<&PgPool> {
        match self {
            Storage::Postgres(p) => Some(p),
            Storage::Sqlite(_) => None,
        }
    }

    pub fn sqlite_pool(&self) -> Option<&SqlitePool> {
        match self {
            Storage::Sqlite(p) => Some(p),
            Storage::Postgres(_) => None,
        }
    }

    pub async fn migrate(&self) -> Result<()> {
        match self {
            Storage::Postgres(pool) => {
                let sql = include_str!("../../../migrations/001_init.sql");
                sqlx::raw_sql(sql).execute(pool).await?;
            }
            Storage::Sqlite(pool) => {
                let sql = include_str!("../../../migrations/sqlite.sql");
                sqlx::raw_sql(sql).execute(pool).await?;
            }
        }
        Ok(())
    }

    pub async fn upsert_agent(&self, id: &str, req: &RegisterRequest) -> Result<()> {
        match self {
            Storage::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO agents (id, hostname, os, arch, agent_version, labels)
                     VALUES ($1, $2, $3, $4, $5, $6)
                     ON CONFLICT (id) DO UPDATE SET
                        hostname = EXCLUDED.hostname,
                        agent_version = EXCLUDED.agent_version,
                        last_seen_at = NOW(),
                        status = 'online'",
                )
                .bind(id)
                .bind(&req.hostname)
                .bind(&req.os)
                .bind(&req.arch)
                .bind(&req.agent_version)
                .bind(&req.labels)
                .execute(pool)
                .await?;
            }
            Storage::Sqlite(pool) => {
                let labels = serde_json::to_string(&req.labels)?;
                sqlx::query(
                    "INSERT INTO agents (id, hostname, os, arch, agent_version, labels)
                     VALUES ($1, $2, $3, $4, $5, $6)
                     ON CONFLICT (id) DO UPDATE SET
                        hostname = EXCLUDED.hostname,
                        agent_version = EXCLUDED.agent_version,
                        last_seen_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                        status = 'online'",
                )
                .bind(id)
                .bind(&req.hostname)
                .bind(&req.os)
                .bind(&req.arch)
                .bind(&req.agent_version)
                .bind(&labels)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn insert_metrics(&self, payload: &MetricsPayload) -> Result<()> {
        let time = DateTime::<Utc>::from_timestamp_millis(payload.timestamp_ms as i64).unwrap_or_else(Utc::now);

        let (cpu_total, cpu_cores_vec, load1, load5, load15) = match &payload.cpu {
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

        match self {
            Storage::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO metrics (time, agent_id, cpu_total_bp, cpu_cores_bp, load_1m, load_5m, load_15m,
                        mem_total, mem_used, mem_available, swap_total, swap_used, cgroup_limit, cgroup_usage,
                        uptime_secs, procs_total, procs_running, battery_pct)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)",
                )
                .bind(time)
                .bind(&payload.agent_id)
                .bind(cpu_total)
                .bind(&cpu_cores_vec)
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
                .execute(pool)
                .await?;

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
                    .execute(pool)
                    .await?;
                }

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
                    .execute(pool)
                    .await?;
                }

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
                    .execute(pool)
                    .await?;
                }

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
                    .execute(pool)
                    .await?;
                }

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
                    .execute(pool)
                    .await?;
                }

                for temp in &payload.temperatures {
                    sqlx::query(
                        "INSERT INTO temperature_metrics (time, agent_id, zone, temp_mc)
                         VALUES ($1, $2, $3, $4)",
                    )
                    .bind(time)
                    .bind(&payload.agent_id)
                    .bind(&temp.zone)
                    .bind(temp.temp_mc)
                    .execute(pool)
                    .await?;
                }

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
                    .execute(pool)
                    .await?;
                }

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
                    .execute(pool)
                    .await?;
                }

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
                    .execute(pool)
                    .await?;
                }

                for conn in &payload.connections {
                    sqlx::query(
                        "INSERT INTO connection_metrics (time, agent_id, port, service, active_count, remote_ips)
                         VALUES ($1, $2, $3, $4, $5, $6)",
                    )
                    .bind(time)
                    .bind(&payload.agent_id)
                    .bind(conn.port as i32)
                    .bind(&conn.service)
                    .bind(conn.active_count as i32)
                    .bind(&conn.remote_ips)
                    .execute(pool)
                    .await?;
                }

                sqlx::query("UPDATE agents SET last_seen_at = NOW(), status = 'online' WHERE id = $1")
                    .bind(&payload.agent_id)
                    .execute(pool)
                    .await?;
            }
            Storage::Sqlite(pool) => {
                let time_str = time.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
                let cpu_cores_json = cpu_cores_vec.as_ref().map(|v| serde_json::to_string(v).unwrap_or_default());

                sqlx::query(
                    "INSERT INTO metrics (time, agent_id, cpu_total_bp, cpu_cores_bp, load_1m, load_5m, load_15m,
                        mem_total, mem_used, mem_available, swap_total, swap_used, cgroup_limit, cgroup_usage,
                        uptime_secs, procs_total, procs_running, battery_pct)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)",
                )
                .bind(&time_str)
                .bind(&payload.agent_id)
                .bind(cpu_total)
                .bind(&cpu_cores_json)
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
                .execute(pool)
                .await?;

                for net in &payload.net {
                    sqlx::query(
                        "INSERT INTO net_metrics (time, agent_id, interface, rx_bytes, tx_bytes, rx_packets, tx_packets, rx_errors, tx_errors)
                         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                    )
                    .bind(&time_str)
                    .bind(&payload.agent_id)
                    .bind(&net.interface)
                    .bind(net.rx_bytes_sec as i64)
                    .bind(net.tx_bytes_sec as i64)
                    .bind(net.rx_packets_sec as i64)
                    .bind(net.tx_packets_sec as i64)
                    .bind(net.rx_errors as i64)
                    .bind(net.tx_errors as i64)
                    .execute(pool)
                    .await?;
                }

                for ctr in &payload.containers {
                    sqlx::query(
                        "INSERT INTO container_metrics (time, agent_id, container_id, container_name, image, state, cpu_bp, mem_usage, mem_limit, net_rx_bytes, net_tx_bytes)
                         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
                    )
                    .bind(&time_str)
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
                    .execute(pool)
                    .await?;
                }

                for disk in &payload.disks {
                    sqlx::query(
                        "INSERT INTO disk_metrics (time, agent_id, mount_point, total_bytes, used_bytes)
                         VALUES ($1, $2, $3, $4, $5)",
                    )
                    .bind(&time_str)
                    .bind(&payload.agent_id)
                    .bind(&disk.mount_point)
                    .bind(disk.total_bytes as i64)
                    .bind(disk.used_bytes as i64)
                    .execute(pool)
                    .await?;
                }

                for dio in &payload.disk_io {
                    sqlx::query(
                        "INSERT INTO disk_io_metrics (time, agent_id, device, read_bytes, write_bytes, read_ops, write_ops)
                         VALUES ($1, $2, $3, $4, $5, $6, $7)",
                    )
                    .bind(&time_str)
                    .bind(&payload.agent_id)
                    .bind(&dio.device)
                    .bind(dio.read_bytes as i64)
                    .bind(dio.write_bytes as i64)
                    .bind(dio.read_ops as i64)
                    .bind(dio.write_ops as i64)
                    .execute(pool)
                    .await?;
                }

                for dh in &payload.disk_health {
                    sqlx::query(
                        "INSERT INTO disk_health (time, agent_id, device, model, temp_c, power_on_hours, healthy)
                         VALUES ($1, $2, $3, $4, $5, $6, $7)",
                    )
                    .bind(&time_str)
                    .bind(&payload.agent_id)
                    .bind(&dh.device)
                    .bind(&dh.model)
                    .bind(dh.temp_c as i32)
                    .bind(dh.power_on_hours as i32)
                    .bind(dh.healthy as i32)
                    .execute(pool)
                    .await?;
                }

                for temp in &payload.temperatures {
                    sqlx::query(
                        "INSERT INTO temperature_metrics (time, agent_id, zone, temp_mc)
                         VALUES ($1, $2, $3, $4)",
                    )
                    .bind(&time_str)
                    .bind(&payload.agent_id)
                    .bind(&temp.zone)
                    .bind(temp.temp_mc)
                    .execute(pool)
                    .await?;
                }

                for gpu in &payload.gpus {
                    sqlx::query(
                        "INSERT INTO gpu_metrics (time, agent_id, gpu_index, name, utilization_pct, mem_used, mem_total, temp_c, power_watts)
                         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                    )
                    .bind(&time_str)
                    .bind(&payload.agent_id)
                    .bind(gpu.index as i32)
                    .bind(&gpu.name)
                    .bind(gpu.utilization_pct as i32)
                    .bind(gpu.mem_used as i64)
                    .bind(gpu.mem_total as i64)
                    .bind(gpu.temp_c)
                    .bind(gpu.power_watts as i32)
                    .execute(pool)
                    .await?;
                }

                for proc in &payload.top_processes {
                    sqlx::query(
                        "INSERT INTO process_metrics (time, agent_id, pid, name, cpu_bp, rss_bytes)
                         VALUES ($1, $2, $3, $4, $5, $6)",
                    )
                    .bind(&time_str)
                    .bind(&payload.agent_id)
                    .bind(proc.pid as i32)
                    .bind(&proc.name)
                    .bind(proc.cpu_bp as i32)
                    .bind(proc.rss_bytes as i64)
                    .execute(pool)
                    .await?;
                }

                for sess in &payload.ssh_sessions {
                    sqlx::query(
                        "INSERT INTO ssh_sessions (time, agent_id, username, remote_ip, tty, pid)
                         VALUES ($1, $2, $3, $4, $5, $6)",
                    )
                    .bind(&time_str)
                    .bind(&payload.agent_id)
                    .bind(&sess.user)
                    .bind(&sess.remote_ip)
                    .bind(&sess.tty)
                    .bind(sess.pid as i32)
                    .execute(pool)
                    .await?;
                }

                for conn in &payload.connections {
                    let ips_json = serde_json::to_string(&conn.remote_ips).unwrap_or_default();
                    sqlx::query(
                        "INSERT INTO connection_metrics (time, agent_id, port, service, active_count, remote_ips)
                         VALUES ($1, $2, $3, $4, $5, $6)",
                    )
                    .bind(&time_str)
                    .bind(&payload.agent_id)
                    .bind(conn.port as i32)
                    .bind(&conn.service)
                    .bind(conn.active_count as i32)
                    .bind(&ips_json)
                    .execute(pool)
                    .await?;
                }

                sqlx::query("UPDATE agents SET last_seen_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), status = 'online' WHERE id = $1")
                    .bind(&payload.agent_id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn get_agent_version(&self, agent_id: &str) -> Result<String> {
        let row: Option<(String,)> = match self {
            Storage::Postgres(pool) => {
                sqlx::query_as("SELECT COALESCE(agent_version,'') FROM agents WHERE id = $1")
                    .bind(agent_id)
                    .fetch_optional(pool)
                    .await?
            }
            Storage::Sqlite(pool) => {
                sqlx::query_as("SELECT COALESCE(agent_version,'') FROM agents WHERE id = $1")
                    .bind(agent_id)
                    .fetch_optional(pool)
                    .await?
            }
        };
        Ok(row.map(|r| r.0).unwrap_or_default())
    }

    pub async fn get_offline_agents(&self, timeout_secs: i64) -> Result<Vec<String>> {
        let rows: Vec<(String,)> = match self {
            Storage::Postgres(pool) => {
                sqlx::query_as(
                    "SELECT id FROM agents WHERE last_seen_at < NOW() - make_interval(secs => $1) AND COALESCE(status, 'online') = 'online'",
                )
                .bind(timeout_secs as f64)
                .fetch_all(pool)
                .await?
            }
            Storage::Sqlite(pool) => {
                sqlx::query_as(
                    "SELECT id FROM agents WHERE last_seen_at < datetime('now', '-' || $1 || ' seconds') AND COALESCE(status, 'online') = 'online'",
                )
                .bind(timeout_secs)
                .fetch_all(pool)
                .await?
            }
        };
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    pub async fn set_agent_status(&self, agent_id: &str, status: &str) -> Result<()> {
        match self {
            Storage::Postgres(pool) => {
                sqlx::query("UPDATE agents SET status = $2 WHERE id = $1")
                    .bind(agent_id)
                    .bind(status)
                    .execute(pool)
                    .await?;
            }
            Storage::Sqlite(pool) => {
                sqlx::query("UPDATE agents SET status = $2 WHERE id = $1")
                    .bind(agent_id)
                    .bind(status)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    // REST API helper: list agents
    pub async fn list_agents(&self) -> Result<Vec<serde_json::Value>> {
        match self {
            Storage::Postgres(pool) => {
                let rows: Vec<(String, String, String, String, String, String)> = sqlx::query_as(
                    "SELECT id, hostname, os, arch, agent_version, COALESCE(status,'online') FROM agents ORDER BY registered_at DESC",
                )
                .fetch_all(pool)
                .await?;
                Ok(rows
                    .iter()
                    .map(|a| {
                        serde_json::json!({
                            "id": a.0, "hostname": a.1, "os": a.2, "arch": a.3, "version": a.4, "status": a.5,
                        })
                    })
                    .collect())
            }
            Storage::Sqlite(pool) => {
                let rows: Vec<(String, String, String, String, String, String)> = sqlx::query_as(
                    "SELECT id, hostname, os, arch, agent_version, COALESCE(status,'online') FROM agents ORDER BY registered_at DESC",
                )
                .fetch_all(pool)
                .await?;
                Ok(rows
                    .iter()
                    .map(|a| {
                        serde_json::json!({
                            "id": a.0, "hostname": a.1, "os": a.2, "arch": a.3, "version": a.4, "status": a.5,
                        })
                    })
                    .collect())
            }
        }
    }

    // REST API helper: get recent metrics for an agent
    #[allow(clippy::type_complexity)]
    pub async fn get_agent_metrics(&self, agent_id: &str) -> Result<Vec<serde_json::Value>> {
        match self {
            Storage::Postgres(pool) => {
                let rows: Vec<(String, Option<i32>, Option<i64>, Option<i64>, Option<i64>)> = sqlx::query_as(
                    "SELECT time::text, cpu_total_bp, mem_used, mem_total, uptime_secs FROM metrics WHERE agent_id = $1 ORDER BY time DESC LIMIT 20",
                )
                .bind(agent_id)
                .fetch_all(pool)
                .await?;
                Ok(rows
                    .iter()
                    .map(|m| {
                        serde_json::json!({
                            "time": m.0,
                            "cpu_pct": m.1.map(|v| v as f64 / 100.0),
                            "mem_used_mb": m.2.map(|v| v / 1_048_576),
                            "mem_total_mb": m.3.map(|v| v / 1_048_576),
                            "uptime_secs": m.4,
                        })
                    })
                    .collect())
            }
            Storage::Sqlite(pool) => {
                let rows: Vec<(String, Option<i32>, Option<i64>, Option<i64>, Option<i64>)> = sqlx::query_as(
                    "SELECT time, cpu_total_bp, mem_used, mem_total, uptime_secs FROM metrics WHERE agent_id = $1 ORDER BY time DESC LIMIT 20",
                )
                .bind(agent_id)
                .fetch_all(pool)
                .await?;
                Ok(rows
                    .iter()
                    .map(|m| {
                        serde_json::json!({
                            "time": m.0,
                            "cpu_pct": m.1.map(|v| v as f64 / 100.0),
                            "mem_used_mb": m.2.map(|v| v / 1_048_576),
                            "mem_total_mb": m.3.map(|v| v / 1_048_576),
                            "uptime_secs": m.4,
                        })
                    })
                    .collect())
            }
        }
    }

    // REST API helper: get agent info
    pub async fn get_agent_info(&self, agent_id: &str) -> Result<Option<(String, String, String)>> {
        match self {
            Storage::Postgres(pool) => Ok(sqlx::query_as::<_, (String, String, String)>(
                "SELECT hostname, COALESCE(status,'online'), last_seen_at::text FROM agents WHERE id = $1",
            )
            .bind(agent_id)
            .fetch_optional(pool)
            .await?),
            Storage::Sqlite(pool) => Ok(sqlx::query_as::<_, (String, String, String)>(
                "SELECT hostname, COALESCE(status,'online'), last_seen_at FROM agents WHERE id = $1",
            )
            .bind(agent_id)
            .fetch_optional(pool)
            .await?),
        }
    }

    pub async fn get_agent_connections(&self, agent_id: &str) -> Result<Vec<serde_json::Value>> {
        match self {
            Storage::Postgres(pool) => {
                let rows: Vec<(i32, String, i32, Vec<String>)> = sqlx::query_as(
                    "SELECT DISTINCT ON (port) port, service, active_count, remote_ips
                     FROM connection_metrics WHERE agent_id = $1 ORDER BY port, time DESC",
                )
                .bind(agent_id)
                .fetch_all(pool)
                .await?;
                Ok(rows
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "port": r.0, "service": r.1, "active_count": r.2, "remote_ips": r.3,
                        })
                    })
                    .collect())
            }
            Storage::Sqlite(pool) => {
                let rows: Vec<(i32, String, i32, String)> = sqlx::query_as(
                    "SELECT port, service, active_count, remote_ips FROM connection_metrics
                     WHERE agent_id = $1 GROUP BY port HAVING time = MAX(time) ORDER BY port",
                )
                .bind(agent_id)
                .fetch_all(pool)
                .await?;
                Ok(rows
                    .iter()
                    .map(|r| {
                        let ips: Vec<String> = serde_json::from_str(&r.3).unwrap_or_default();
                        serde_json::json!({
                            "port": r.0, "service": r.1, "active_count": r.2, "remote_ips": ips,
                        })
                    })
                    .collect())
            }
        }
    }

    #[allow(clippy::type_complexity)]
    pub async fn get_overview_data(&self) -> Result<Vec<serde_json::Value>> {
        match self {
            Storage::Postgres(pool) => {
                let rows: Vec<(String, String, String, String)> = sqlx::query_as(
                    "SELECT id, hostname, COALESCE(status,'online'), COALESCE(agent_version,'') FROM agents ORDER BY registered_at DESC",
                )
                .fetch_all(pool)
                .await?;
                let mut agents = Vec::with_capacity(rows.len());
                for (id, hostname, status, version) in &rows {
                    let metric: Option<(Option<i32>, Option<i64>, Option<i64>, Option<i64>)> = sqlx::query_as(
                        "SELECT cpu_total_bp, mem_used, mem_total, uptime_secs FROM metrics WHERE agent_id = $1 ORDER BY time DESC LIMIT 1",
                    )
                    .bind(id)
                    .fetch_optional(pool)
                    .await?;
                    let (cpu_pct, mem_pct, uptime) = match metric {
                        Some((cpu, mem_used, mem_total, up)) => (
                            cpu.map(|v| v as f64 / 100.0),
                            match (mem_used, mem_total) {
                                (Some(u), Some(t)) if t > 0 => Some(u as f64 * 100.0 / t as f64),
                                _ => None,
                            },
                            up,
                        ),
                        None => (None, None, None),
                    };
                    agents.push(serde_json::json!({
                        "id": id, "hostname": hostname, "status": status, "version": version,
                        "cpu_pct": cpu_pct, "mem_pct": mem_pct, "uptime_secs": uptime,
                    }));
                }
                Ok(agents)
            }
            Storage::Sqlite(pool) => {
                let rows: Vec<(String, String, String, String)> = sqlx::query_as(
                    "SELECT id, hostname, COALESCE(status,'online'), COALESCE(agent_version,'') FROM agents ORDER BY registered_at DESC",
                )
                .fetch_all(pool)
                .await?;
                let mut agents = Vec::with_capacity(rows.len());
                for (id, hostname, status, version) in &rows {
                    let metric: Option<(Option<i32>, Option<i64>, Option<i64>, Option<i64>)> = sqlx::query_as(
                        "SELECT cpu_total_bp, mem_used, mem_total, uptime_secs FROM metrics WHERE agent_id = $1 ORDER BY time DESC LIMIT 1",
                    )
                    .bind(id)
                    .fetch_optional(pool)
                    .await?;
                    let (cpu_pct, mem_pct, uptime) = match metric {
                        Some((cpu, mem_used, mem_total, up)) => (
                            cpu.map(|v| v as f64 / 100.0),
                            match (mem_used, mem_total) {
                                (Some(u), Some(t)) if t > 0 => Some(u as f64 * 100.0 / t as f64),
                                _ => None,
                            },
                            up,
                        ),
                        None => (None, None, None),
                    };
                    agents.push(serde_json::json!({
                        "id": id, "hostname": hostname, "status": status, "version": version,
                        "cpu_pct": cpu_pct, "mem_pct": mem_pct, "uptime_secs": uptime,
                    }));
                }
                Ok(agents)
            }
        }
    }

    #[allow(clippy::type_complexity)]
    pub async fn get_latest_metric(&self, agent_id: &str) -> Result<Option<serde_json::Value>> {
        match self {
            Storage::Postgres(pool) => {
                let row: Option<(String, Option<i32>, Option<i64>, Option<i64>, Option<i64>, Option<i32>)> = sqlx::query_as(
                    "SELECT time::text, cpu_total_bp, mem_used, mem_total, uptime_secs, battery_pct FROM metrics WHERE agent_id = $1 ORDER BY time DESC LIMIT 1",
                )
                .bind(agent_id)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(|(time, cpu, mem_used, mem_total, uptime, battery)| {
                    let mem_used_mb = mem_used.map(|v| v / 1_048_576);
                    let mem_total_mb = mem_total.map(|v| v / 1_048_576);
                    let mem_pct = match (mem_used, mem_total) {
                        (Some(u), Some(t)) if t > 0 => Some(u as f64 * 100.0 / t as f64),
                        _ => None,
                    };
                    serde_json::json!({
                        "agent_id": agent_id, "time": time,
                        "cpu_pct": cpu.map(|v| v as f64 / 100.0),
                        "mem_used_mb": mem_used_mb, "mem_total_mb": mem_total_mb,
                        "mem_pct": mem_pct, "uptime_secs": uptime,
                        "battery_pct": battery,
                    })
                }))
            }
            Storage::Sqlite(pool) => {
                let row: Option<(String, Option<i32>, Option<i64>, Option<i64>, Option<i64>, Option<i32>)> = sqlx::query_as(
                    "SELECT time, cpu_total_bp, mem_used, mem_total, uptime_secs, battery_pct FROM metrics WHERE agent_id = $1 ORDER BY time DESC LIMIT 1",
                )
                .bind(agent_id)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(|(time, cpu, mem_used, mem_total, uptime, battery)| {
                    let mem_used_mb = mem_used.map(|v| v / 1_048_576);
                    let mem_total_mb = mem_total.map(|v| v / 1_048_576);
                    let mem_pct = match (mem_used, mem_total) {
                        (Some(u), Some(t)) if t > 0 => Some(u as f64 * 100.0 / t as f64),
                        _ => None,
                    };
                    serde_json::json!({
                        "agent_id": agent_id, "time": time,
                        "cpu_pct": cpu.map(|v| v as f64 / 100.0),
                        "mem_used_mb": mem_used_mb, "mem_total_mb": mem_total_mb,
                        "mem_pct": mem_pct, "uptime_secs": uptime,
                        "battery_pct": battery,
                    })
                }))
            }
        }
    }

    pub async fn get_agent_disks(&self, agent_id: &str) -> Result<Vec<serde_json::Value>> {
        match self {
            Storage::Postgres(pool) => {
                let rows: Vec<(String, Option<i64>, Option<i64>)> = sqlx::query_as(
                    "SELECT DISTINCT ON (mount_point) mount_point, total_bytes, used_bytes
                     FROM disk_metrics WHERE agent_id = $1 ORDER BY mount_point, time DESC",
                )
                .bind(agent_id)
                .fetch_all(pool)
                .await?;
                Ok(rows.iter().map(|(mp, total, used)| {
                    let total_gb = total.map(|v| v as f64 / 1_073_741_824.0);
                    let used_gb = used.map(|v| v as f64 / 1_073_741_824.0);
                    let used_pct = match (used, total) {
                        (Some(u), Some(t)) if *t > 0 => Some(*u as f64 * 100.0 / *t as f64),
                        _ => None,
                    };
                    serde_json::json!({"mount_point": mp, "total_gb": total_gb, "used_gb": used_gb, "used_pct": used_pct})
                }).collect())
            }
            Storage::Sqlite(pool) => {
                let rows: Vec<(String, Option<i64>, Option<i64>)> = sqlx::query_as(
                    "SELECT mount_point, total_bytes, used_bytes FROM disk_metrics
                     WHERE agent_id = $1 GROUP BY mount_point HAVING time = MAX(time)",
                )
                .bind(agent_id)
                .fetch_all(pool)
                .await?;
                Ok(rows.iter().map(|(mp, total, used)| {
                    let total_gb = total.map(|v| v as f64 / 1_073_741_824.0);
                    let used_gb = used.map(|v| v as f64 / 1_073_741_824.0);
                    let used_pct = match (used, total) {
                        (Some(u), Some(t)) if *t > 0 => Some(*u as f64 * 100.0 / *t as f64),
                        _ => None,
                    };
                    serde_json::json!({"mount_point": mp, "total_gb": total_gb, "used_gb": used_gb, "used_pct": used_pct})
                }).collect())
            }
        }
    }

    pub async fn get_agent_network(&self, agent_id: &str) -> Result<Vec<serde_json::Value>> {
        match self {
            Storage::Postgres(pool) => {
                let rows: Vec<(String, Option<i64>, Option<i64>)> = sqlx::query_as(
                    "SELECT DISTINCT ON (interface) interface, rx_bytes, tx_bytes
                     FROM net_metrics WHERE agent_id = $1 ORDER BY interface, time DESC",
                )
                .bind(agent_id)
                .fetch_all(pool)
                .await?;
                Ok(rows
                    .iter()
                    .map(|(name, rx, tx)| serde_json::json!({"name": name, "rx_bytes": rx, "tx_bytes": tx}))
                    .collect())
            }
            Storage::Sqlite(pool) => {
                let rows: Vec<(String, Option<i64>, Option<i64>)> = sqlx::query_as(
                    "SELECT interface, rx_bytes, tx_bytes FROM net_metrics
                     WHERE agent_id = $1 GROUP BY interface HAVING time = MAX(time)",
                )
                .bind(agent_id)
                .fetch_all(pool)
                .await?;
                Ok(rows
                    .iter()
                    .map(|(name, rx, tx)| serde_json::json!({"name": name, "rx_bytes": rx, "tx_bytes": tx}))
                    .collect())
            }
        }
    }

    #[allow(clippy::type_complexity)]
    pub async fn get_agent_containers(&self, agent_id: &str) -> Result<Vec<serde_json::Value>> {
        match self {
            Storage::Postgres(pool) => {
                let rows: Vec<(String, String, Option<String>, Option<String>, Option<i32>, Option<i64>, Option<i64>)> = sqlx::query_as(
                    "SELECT DISTINCT ON (container_id) container_id, container_name, image, state, cpu_bp, mem_usage, mem_limit
                     FROM container_metrics WHERE agent_id = $1 ORDER BY container_id, time DESC",
                )
                .bind(agent_id)
                .fetch_all(pool)
                .await?;
                Ok(rows
                    .iter()
                    .map(|(id, name, image, state, cpu_bp, mem_usage, _mem_limit)| {
                        serde_json::json!({
                            "id": id, "name": name, "image": image, "state": state,
                            "cpu_pct": cpu_bp.map(|v| v as f64 / 100.0),
                            "mem_mb": mem_usage.map(|v| v / 1_048_576),
                        })
                    })
                    .collect())
            }
            Storage::Sqlite(pool) => {
                let rows: Vec<(String, String, Option<String>, Option<String>, Option<i32>, Option<i64>, Option<i64>)> =
                    sqlx::query_as(
                        "SELECT container_id, container_name, image, state, cpu_bp, mem_usage, mem_limit
                     FROM container_metrics WHERE agent_id = $1 GROUP BY container_id HAVING time = MAX(time)",
                    )
                    .bind(agent_id)
                    .fetch_all(pool)
                    .await?;
                Ok(rows
                    .iter()
                    .map(|(id, name, image, state, cpu_bp, mem_usage, _mem_limit)| {
                        serde_json::json!({
                            "id": id, "name": name, "image": image, "state": state,
                            "cpu_pct": cpu_bp.map(|v| v as f64 / 100.0),
                            "mem_mb": mem_usage.map(|v| v / 1_048_576),
                        })
                    })
                    .collect())
            }
        }
    }

    pub async fn get_agent_processes(&self, agent_id: &str) -> Result<Vec<serde_json::Value>> {
        match self {
            Storage::Postgres(pool) => {
                let rows: Vec<(i32, String, Option<i32>, Option<i64>)> = sqlx::query_as(
                    "SELECT pid, name, cpu_bp, rss_bytes FROM process_metrics
                     WHERE agent_id = $1 AND time = (SELECT MAX(time) FROM process_metrics WHERE agent_id = $1)
                     ORDER BY cpu_bp DESC NULLS LAST",
                )
                .bind(agent_id)
                .fetch_all(pool)
                .await?;
                Ok(rows
                    .iter()
                    .map(|(pid, name, cpu_bp, rss)| {
                        serde_json::json!({
                            "pid": pid, "name": name,
                            "cpu_pct": cpu_bp.map(|v| v as f64 / 100.0),
                            "rss_mb": rss.map(|v| v / 1_048_576),
                        })
                    })
                    .collect())
            }
            Storage::Sqlite(pool) => {
                let rows: Vec<(i32, String, Option<i32>, Option<i64>)> = sqlx::query_as(
                    "SELECT pid, name, cpu_bp, rss_bytes FROM process_metrics
                     WHERE agent_id = $1 AND time = (SELECT MAX(time) FROM process_metrics WHERE agent_id = $1)
                     ORDER BY cpu_bp DESC",
                )
                .bind(agent_id)
                .fetch_all(pool)
                .await?;
                Ok(rows
                    .iter()
                    .map(|(pid, name, cpu_bp, rss)| {
                        serde_json::json!({
                            "pid": pid, "name": name,
                            "cpu_pct": cpu_bp.map(|v| v as f64 / 100.0),
                            "rss_mb": rss.map(|v| v / 1_048_576),
                        })
                    })
                    .collect())
            }
        }
    }

    #[allow(clippy::type_complexity)]
    pub async fn get_agent_gpu(&self, agent_id: &str) -> Result<Vec<serde_json::Value>> {
        match self {
            Storage::Postgres(pool) => {
                let rows: Vec<(i32, Option<String>, Option<i32>, Option<i64>, Option<i64>, Option<i32>, Option<i32>)> = sqlx::query_as(
                    "SELECT DISTINCT ON (gpu_index) gpu_index, name, utilization_pct, mem_used, mem_total, temp_c, power_watts
                     FROM gpu_metrics WHERE agent_id = $1 ORDER BY gpu_index, time DESC",
                )
                .bind(agent_id)
                .fetch_all(pool)
                .await?;
                Ok(rows
                    .iter()
                    .map(|(idx, name, util, mem_used, mem_total, temp, power)| {
                        serde_json::json!({
                            "index": idx, "name": name, "utilization_pct": util,
                            "mem_used_mb": mem_used.map(|v| v / 1_048_576),
                            "mem_total_mb": mem_total.map(|v| v / 1_048_576),
                            "temp_c": temp, "power_watts": power,
                        })
                    })
                    .collect())
            }
            Storage::Sqlite(pool) => {
                let rows: Vec<(i32, Option<String>, Option<i32>, Option<i64>, Option<i64>, Option<i32>, Option<i32>)> =
                    sqlx::query_as(
                        "SELECT gpu_index, name, utilization_pct, mem_used, mem_total, temp_c, power_watts
                     FROM gpu_metrics WHERE agent_id = $1 GROUP BY gpu_index HAVING time = MAX(time)",
                    )
                    .bind(agent_id)
                    .fetch_all(pool)
                    .await?;
                Ok(rows
                    .iter()
                    .map(|(idx, name, util, mem_used, mem_total, temp, power)| {
                        serde_json::json!({
                            "index": idx, "name": name, "utilization_pct": util,
                            "mem_used_mb": mem_used.map(|v| v / 1_048_576),
                            "mem_total_mb": mem_total.map(|v| v / 1_048_576),
                            "temp_c": temp, "power_watts": power,
                        })
                    })
                    .collect())
            }
        }
    }

    pub async fn delete_agent(&self, agent_id: &str) -> Result<()> {
        let tables = [
            "metrics",
            "net_metrics",
            "container_metrics",
            "disk_metrics",
            "disk_io_metrics",
            "disk_health",
            "temperature_metrics",
            "gpu_metrics",
            "process_metrics",
            "ssh_sessions",
            "connection_metrics",
            "alerts_history",
        ];
        match self {
            Storage::Postgres(pool) => {
                for table in tables {
                    let sql = format!("DELETE FROM {table} WHERE agent_id = $1");
                    sqlx::query(&sql).bind(agent_id).execute(pool).await?;
                }
                sqlx::query("DELETE FROM agents WHERE id = $1").bind(agent_id).execute(pool).await?;
            }
            Storage::Sqlite(pool) => {
                for table in tables {
                    let sql = format!("DELETE FROM {table} WHERE agent_id = $1");
                    sqlx::query(&sql).bind(agent_id).execute(pool).await?;
                }
                sqlx::query("DELETE FROM agents WHERE id = $1").bind(agent_id).execute(pool).await?;
            }
        }
        Ok(())
    }

    pub async fn insert_alert(&self, agent_id: &str, alert_type: &str, message: &str) -> Result<()> {
        match self {
            Storage::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO alerts_history (time, agent_id, alert_type, message) VALUES (NOW(), $1, $2, $3)",
                )
                .bind(agent_id)
                .bind(alert_type)
                .bind(message)
                .execute(pool)
                .await?;
            }
            Storage::Sqlite(pool) => {
                sqlx::query("INSERT INTO alerts_history (agent_id, alert_type, message) VALUES ($1, $2, $3)")
                    .bind(agent_id)
                    .bind(alert_type)
                    .bind(message)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn get_alerts_history(&self) -> Result<Vec<serde_json::Value>> {
        match self {
            Storage::Postgres(pool) => {
                let rows: Vec<(String, String, String, String)> = sqlx::query_as(
                    "SELECT time::text, agent_id, alert_type, message FROM alerts_history ORDER BY time DESC LIMIT 50",
                )
                .fetch_all(pool)
                .await?;
                Ok(rows.iter().map(|(time, agent_id, atype, msg)| {
                    serde_json::json!({"time": time, "agent_id": agent_id, "alert_type": atype, "message": msg})
                }).collect())
            }
            Storage::Sqlite(pool) => {
                let rows: Vec<(String, String, String, String)> = sqlx::query_as(
                    "SELECT time, agent_id, alert_type, message FROM alerts_history ORDER BY time DESC LIMIT 50",
                )
                .fetch_all(pool)
                .await?;
                Ok(rows.iter().map(|(time, agent_id, atype, msg)| {
                    serde_json::json!({"time": time, "agent_id": agent_id, "alert_type": atype, "message": msg})
                }).collect())
            }
        }
    }

    pub async fn delete_old_data(&self, retention_days: i64) -> Result<()> {
        let tables = [
            "metrics",
            "net_metrics",
            "container_metrics",
            "disk_metrics",
            "disk_io_metrics",
            "disk_health",
            "temperature_metrics",
            "gpu_metrics",
            "process_metrics",
            "ssh_sessions",
            "connection_metrics",
        ];

        match self {
            Storage::Postgres(pool) => {
                for table in tables {
                    let sql = format!("DELETE FROM {table} WHERE time < NOW() - make_interval(days => $1)");
                    sqlx::query(&sql).bind(retention_days as i32).execute(pool).await?;
                }
            }
            Storage::Sqlite(pool) => {
                for table in tables {
                    let sql = format!("DELETE FROM {table} WHERE time < datetime('now', '-' || $1 || ' days')");
                    sqlx::query(&sql).bind(retention_days).execute(pool).await?;
                }
            }
        }
        Ok(())
    }
}
