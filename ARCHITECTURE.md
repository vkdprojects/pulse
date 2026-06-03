# Vixpi Pulse — Análise Técnica e Validação Arquitetural

> Documento gerado a partir de pesquisa ativa com fontes verificáveis.
> Data: 2026-06-02

---

## 1. gRPC/tonic + mTLS: Validação de Viabilidade

### Veredicto: ✅ VIÁVEL dentro de 2-5 MB idle

| Componente | Custo de Memória |
|---|---|
| tokio `current_thread` runtime | ~1-2 MB |
| rustls sessão TLS (mTLS) | ~13 KB |
| hyper HTTP/2 connection | ~16 KB |
| tonic overhead (codec, metadata) | ~2-4 KB |
| **Total cliente idle (1 conexão)** | **~2-3 MB** |

**Dados-chave:**
- rustls usa ~13 KiB/sessão vs ~69 KiB do OpenSSL (5x mais eficiente). [Fonte: Prossimo/ISRG, Jan 2024](https://www.memorysafety.org/blog/rustls-performance/)
- Um servidor tonic multi-service roda com <5 MB total em htop. [Fonte: users.rust-lang.org, Jun 2025](https://users.rust-lang.org/t/help-grpc-server-stops-responding/130564)
- Full handshake TLS 1.3 com aws-lc-rs: latência ~2x menor que OpenSSL. [Fonte: Prossimo, Out 2024](https://www.memorysafety.org/blog/rustls-performance-outperforms/)

### Recomendações para o agente:

```rust
// Economiza ~2 MB vs multi-thread
tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
```

- **Crypto provider**: `rustls` + `aws-lc-rs` (performance superior, mantido pela AWS)
- **Não usar**: `native-tls` (wrapper OpenSSL, 5x mais memória por sessão)
- **Cuidado**: tracing-subscriber pode consumir 6+ MB sozinho. Usar log minimal ou desabilitar em produção.

### Lado Master (milhares de conexões):

| Conexões | RAM estimada |
|---|---|
| 100 agentes | ~5-8 MB |
| 1.000 agentes | ~50-80 MB |
| 10.000 agentes | ~500-800 MB |

Escala linearmente. rustls mantém throughput até 80 cores sem degradação. [Fonte: Prossimo, Mai 2025](https://www.memorysafety.org/blog/rustls-server-perf/)

---

## 2. UPX em Binários Rust musl

### Veredicto: ❌ NÃO USAR para daemon

**Problema fundamental:** UPX descomprime o binário inteiro na RAM como páginas anônimas (dirty/private). O kernel NÃO pode descartá-las — precisa de swap para reclamá-las.

| Cenário | Sem UPX | Com UPX |
|---|---|---|
| nushell (Rust) | 3 MB RSS | **32 MB RSS** (10x) |
| GraalVM native | 50 MB RSS | **100 MB RSS** (2x) |
| Páginas descartáveis pelo kernel? | ✅ Sim (file-backed) | ❌ Não (anonymous) |
| Compartilhamento entre instâncias | ✅ Via page cache | ❌ Cópias privadas |

[Fonte: nushell/nushell#4131](https://github.com/nushell/nushell/issues/4131), [upx/upx#459](https://github.com/upx/upx/issues/459)

### Estratégia correta para binário mínimo:

```toml
[profile.release]
opt-level = "z"      # Otimiza para tamanho
lto = true           # Fat LTO (máxima redução)
codegen-units = 1    # Máxima otimização cross-crate
panic = "abort"      # Remove unwind tables
strip = true         # Remove símbolos
```

**Resultado esperado**: binário de ~8-15 MB sem nenhum overhead de runtime (vs ~50 MB sem otimizações).

---

## 3. Crate `sysinfo`

### Veredicto: ❌ NÃO USAR — substituir por leitura direta de /proc

**Problemas críticos para nosso caso de uso:**

1. **Não lê cgroups**: `System::total_memory()` retorna memória do HOST, não do container. Inútil em LXC/Docker.
2. **Regressão em containers**: Issue #1595 (Out 2025) — métricas de processo falham em containers.
3. **Overhead desnecessário**: mantém FDs abertos, multithread por default (rayon), HashMap de todos os processos.
4. **Sem dados de hardware em VMs**: README confirma que `/sys/class/hwmon` e `/sys/class/thermal` não funcionam em ambientes virtualizados.

### Abordagem recomendada: leitura direta

| Métrica | Fonte | Complexidade |
|---|---|---|
| CPU usage | `/proc/stat` | Parsing de 2 linhas |
| Memória (host) | `/proc/meminfo` | Parsing de 3 campos |
| Memória (cgroup v2) | `/sys/fs/cgroup/memory.max` + `memory.current` | 2 reads |
| Memória (cgroup v1) | `/sys/fs/cgroup/memory/memory.limit_in_bytes` | 1 read |
| Disco | `/proc/diskstats` + `/proc/mounts` | Parsing simples |
| Rede | `/proc/net/dev` | 1 read por interface |
| Load average | `/proc/loadavg` | 1 read |
| Uptime | `/proc/uptime` | 1 read |

**Vantagens:**
- Zero alocação dinâmica (buffers estáticos reutilizáveis)
- Cgroup-aware nativamente
- Sem threads extras
- Sem FDs permanentes
- Total controle do ciclo de coleta

---

## 4. Falhas Identificadas no Plano Original

| Item do plan.md | Problema | Correção |
|---|---|---|
| "Telemetria via crate `sysinfo`" | Não funciona corretamente em containers/LXC | Leitura direta de `/proc` + `/sys/fs/cgroup` |
| "Comprimida com UPX" | 10x mais RSS para daemon | strip + fat-LTO + opt-level=z + panic=abort |
| Nenhuma menção a cgroup awareness | Reportaria dados do host em containers | Detectar cgroup v1/v2 e ler limits |
| Nenhuma menção ao runtime tokio | Multi-thread default consome ~4 MB | Usar `current_thread` no agente |

---

## 5. Draft do Protobuf

```protobuf
syntax = "proto3";
package vixpi.pulse.v1;

// --- Serviços ---

service PulseAgent {
  // Agent → Master: stream contínuo de métricas
  rpc StreamMetrics(stream MetricsPayload) returns (MetricsAck);

  // Master ↔ Agent: comandos bidirecionais
  rpc CommandChannel(stream CommandResponse) returns (stream CommandRequest);

  // Agent → Master: registro inicial do agente
  rpc Register(RegisterRequest) returns (RegisterResponse);
}

// --- Métricas (otimizado para banda) ---

message MetricsPayload {
  // Timestamp Unix em milissegundos (8 bytes vs 16 do google.protobuf.Timestamp)
  fixed64 timestamp_ms = 1;
  string agent_id = 2;

  CpuMetrics cpu = 3;
  MemoryMetrics memory = 4;
  repeated DiskMetrics disks = 5;
  repeated NetMetrics net = 6;
  SystemMetrics system = 7;
}

message CpuMetrics {
  // Percentual de uso por core (0-10000 = 0.00%-100.00%), evita float
  repeated uint32 usage_per_core_bp = 1;
  uint32 usage_total_bp = 2;
  uint32 load_avg_1m_cp = 3;  // centipercent (load * 100)
  uint32 load_avg_5m_cp = 4;
  uint32 load_avg_15m_cp = 5;
}

message MemoryMetrics {
  uint64 total_bytes = 1;
  uint64 used_bytes = 2;
  uint64 available_bytes = 3;
  uint64 swap_total_bytes = 4;
  uint64 swap_used_bytes = 5;
  // Cgroup limits (0 = sem limit / não detectado)
  uint64 cgroup_limit_bytes = 6;
  uint64 cgroup_usage_bytes = 7;
}

message DiskMetrics {
  string mount_point = 1;
  uint64 total_bytes = 2;
  uint64 used_bytes = 3;
  uint64 read_bytes_sec = 4;
  uint64 write_bytes_sec = 5;
}

message NetMetrics {
  string interface = 1;
  uint64 rx_bytes_sec = 2;
  uint64 tx_bytes_sec = 3;
  uint64 rx_packets_sec = 4;
  uint64 tx_packets_sec = 5;
  uint64 rx_errors = 6;
  uint64 tx_errors = 7;
}

message SystemMetrics {
  uint64 uptime_secs = 1;
  uint32 processes_total = 2;
  uint32 processes_running = 3;
}

message MetricsAck {
  bool ok = 1;
  // Servidor pode ajustar intervalo de coleta dinamicamente
  uint32 next_interval_ms = 2;
}

// --- Comandos ---

message CommandRequest {
  string request_id = 1;
  oneof command {
    ExecCommand exec = 2;
    PackageCommand package = 3;
    FileCommand file = 4;
  }
  uint32 timeout_secs = 5;
}

message ExecCommand {
  string program = 1;
  repeated string args = 2;
  map<string, string> env = 3;
  string working_dir = 4;
}

message PackageCommand {
  enum Action {
    INSTALL = 0;
    REMOVE = 1;
    UPDATE = 2;
  }
  Action action = 1;
  repeated string packages = 2;
}

message FileCommand {
  enum Action {
    READ = 0;
    WRITE = 1;
    APPEND = 2;
  }
  Action action = 1;
  string path = 2;
  bytes content = 3;  // Para WRITE/APPEND
  uint64 offset = 4;  // Para READ parcial
  uint64 limit = 5;   // Para READ parcial
}

message CommandResponse {
  string request_id = 1;
  oneof payload {
    OutputChunk output = 2;
    CommandResult result = 3;
  }
}

message OutputChunk {
  enum Stream {
    STDOUT = 0;
    STDERR = 1;
  }
  Stream stream = 1;
  bytes data = 2;
}

message CommandResult {
  int32 exit_code = 1;
  bool timed_out = 2;
  string error = 3;  // Erro interno (ex: processo não encontrado)
}

// --- Registro ---

message RegisterRequest {
  string hostname = 1;
  string os = 2;
  string arch = 3;
  string agent_version = 4;
  repeated string labels = 5;  // Tags para agrupamento
}

message RegisterResponse {
  string agent_id = 1;
  uint32 metrics_interval_ms = 2;
  bytes ca_cert = 3;  // CA cert para validar o master
}
```

### Decisões de design do Protobuf:

| Decisão | Motivo |
|---|---|
| `fixed64` para timestamps | 8 bytes fixos vs varint encoding que pode ser maior para timestamps grandes |
| `uint32` basis points para CPU | Evita float (4 bytes IEEE754 vs 2 bytes varint para valores <16384) |
| `oneof` em CommandRequest | Multiplexação sem overhead — só 1 campo serializado |
| `bytes` para output chunks | Evita UTF-8 validation overhead do `string` |
| Campos 1-15 para hot fields | Encoding em 1 byte (vs 2 bytes para fields 16-2047) |
| `MetricsAck.next_interval_ms` | Controle de backpressure server-driven |
| Separação ExecCommand/PackageCommand | Type-safety + o Master pode ter policies diferentes por tipo |

### Estimativa de tamanho por mensagem:

- `MetricsPayload` típico (4 cores, 2 discos, 1 NIC): ~120-180 bytes
- `OutputChunk` (4 KB de stdout): ~4.1 KB
- `CommandRequest` (exec simples): ~50-80 bytes

A 10s de intervalo, um agente gera ~1 KB/min de tráfego de métricas.

---

## 6. Stack Final Recomendada

### Agente (`vixpi-pulse`)

| Componente | Escolha | Motivo |
|---|---|---|
| Runtime | tokio `current_thread` | -2 MB vs multi-thread |
| gRPC | tonic + prost | Ecossistema padrão, viável em <3 MB |
| TLS | rustls + aws-lc-rs | 5x menos memória que OpenSSL |
| Telemetria | Leitura direta /proc + /sys/fs/cgroup | Cgroup-aware, zero overhead |
| Compilação | musl static + fat-LTO + opt-z + strip | 8-15 MB binário, zero runtime overhead |
| Logging | Minimal (sem tracing-subscriber) | Economiza 6+ MB |

### Master (`vixpi-master`)

| Componente | Escolha | Motivo |
|---|---|---|
| Runtime | tokio multi-thread | Precisa de paralelismo real |
| gRPC | tonic | Já validado para milhares de conexões |
| TLS | rustls + aws-lc-rs | Escala linear até 80 cores |
| DB | sqlx + PostgreSQL | Async, compile-time checked queries |
| CA interna | rcgen + rustls | Geração de certs X.509 em pure Rust |

---

## 7. Riscos e Mitigações

| Risco | Probabilidade | Mitigação |
|---|---|---|
| Memory spike durante prost desserialização | Média | Buffer pool fixo, limitar tamanho max de mensagem |
| Agente perde conexão e acumula métricas | Alta | Ring buffer fixo (drop oldest), reconexão com backoff |
| cgroup v1 vs v2 detection | Certa | Detectar presença de `/sys/fs/cgroup/cgroup.controllers` (v2) |
| Binário musl não linkando com aws-lc-rs | Média | aws-lc-rs requer cmake+C compiler. Fallback: ring |
