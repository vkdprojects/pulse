# Vixpi Pulse: Especificação Arquitetural e Diretrizes de Implementação

## 1. O Contexto e a Problemática
Atualmente, o monitoramento e a gestão remota de servidores (VMs, bare-metal) sofrem com um dilema: ou utilizamos soluções completas e pesadas (como Zabbix, Datadog, New Relic) que consomem recursos significativos de CPU/RAM apenas para existir, ou criamos scripts customizados frágeis baseados em SSH e cronjobs. 

O objetivo do ecossistema **Vixpi** é fornecer um controle de infraestrutura com overhead próximo a zero. Precisamos de um agente (daqui em diante chamado de `vixpi-pulse`) que atue como um "mini-Zabbix" extremamente leve, seguro e autônomo. Ele deve ser capaz de:
1. Coletar telemetria de hardware e rede continuamente.
2. Executar comandos de sistema e instalar pacotes remotamente sob demanda.
3. Consumir o mínimo absoluto de memória e CPU (target de 2MB a 5MB de RAM em idle).

## 2. A Arquitetura Proposta

O sistema adota uma topologia Master/Slave baseada em streaming bidirecional E2EE. Toda a stack será construída em **Rust** para garantir o *memory footprint* mínimo e a ausência de Garbage Collector.

### 2.1. O Agente: `vixpi-pulse` (Slave)
* **Linguagem/Runtime:** Rust com `tokio`.
* **Telemetria:** Leitura direta do kernel via crate `sysinfo` (sem execuções de subprocessos shell para monitoramento básico).
* **Execução:** Módulo `Worker` utilizando `std::process::Command` para rodar instruções (ex: `apt install`, leitura de logs) e fazer o stream de `stdout`/`stderr`.
* **Distribuição:** Compilação estática (`x86_64-unknown-linux-musl`) otimizada com LTO e comprimida utilizando **UPX** para gerar um binário minúsculo.

### 2.2. O Servidor Central: `vixpi-master` (Cérebro)
* **Linguagem/Runtime:** Rust com `tokio`.
* **Persistência:** PostgreSQL via `sqlx` (assíncrono) para inventário e séries temporais.
* **Segurança (CA Interna):** O Master atuará como sua própria Autoridade Certificadora. Ele deve gerar e assinar os certificados dos agentes no momento do provisionamento.

### 2.3. O Protocolo de Comunicação
* **RPC:** **gRPC** via crate `tonic`.
* **Segurança de Transporte:** **mTLS** (Mutual TLS) estrito. O Master valida o agente, e o agente valida o Master. Nenhuma conexão não assinada é aceita.
* **Contratos (Protobuf):**
  * `MetricsStream`: Stream de upload unidirecional (Agente -> Master) com os dados de hardware.
  * `CommandStream`: Stream bidirecional (Master <-> Agente) para envio de tarefas e retorno de logs de execução em tempo real.

---

## 3. Diretriz de Execução para a IA (Seu Papel)

Você atuará como Engenheiro de Software Pleno/Sênior auxiliando o Arquiteto responsável. Antes de gerar qualquer código de implementação, você **DEVE** realizar uma pesquisa profunda e validação científica da arquitetura acima. 

Sua primeira resposta deve conter obrigatoriamente a análise crítica dos seguintes pontos:

1. **Validação do gRPC sobre mTLS em Rust:** Existe algum *overhead* significativo de handshake ou manutenção de túnel no `tonic` que possa estourar nosso limite de memória de 5MB no cliente? Qual o custo de manter milhares de streams abertos no `tokio` (lado Master)?
2. **Compressão UPX em Binários Rust (musl):** Faça uma análise sobre o impacto do UPX no tempo de *startup* e na alocação de memória residente (RSS) ao carregar o binário na RAM do SO. Vale a pena para um daemon de longa duração?
3. **Crate `sysinfo`:** Pesquise as limitações conhecidas desta crate em ambientes de virtualização restrita (ex: LXC ou instâncias EC2 muito pequenas).
4. **Draft do Protobuf:** Apresente uma proposta de esquema `.proto` otimizado para o consumo de banda, desenhando os serviços de Metrics e Commands.

Aja com rigor técnico. Aponte falhas caso existam na arquitetura proposta e sugira as alternativas mais performáticas do ecossistema Rust atual. Aguardo sua análise técnica antes de iniciarmos os *commits*.
