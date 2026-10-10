# Filosofia — Caber no Bolso do Servidor

O Pulse existe com uma tese: monitoring não pode custar mais que o serviço que monitora.

## Princípios

1. **Invisível** — O agent não pode ser perceptível no servidor. 2 MB de RAM. 0% de CPU idle. O operador esquece que existe.

2. **Zero dependências** — Nenhum PostgreSQL, nenhum Docker, nenhum runtime, nenhum config file obrigatório. Um binário. Funciona.

3. **Segurança é default** — TLS automático. CA pin. Token auth. Client cert provisioning. Sem opt-in.

4. **DX é UX** — Se o operador precisa ler 3 páginas de docs pra instalar, falhamos. 1 comando. 5 segundos.

5. **Dados sempre disponíveis** — Reconnect automático. Buffer offline. Retention automática. Sem perda.

## O que isso significa na prática

- Nunca adicionar dependência externa sem justificativa de peso
- Nunca aceitar "funciona mas precisa de Docker" — precisa funcionar com 1 binário
- Nunca usar `unwrap()` em código de produção — handle ou `?`
- Nunca aceitar "precisa reiniciar pra aplicar" — hot-reload ou auto-config
- Nunca criar config que o master pode gerar sozinho

## Hierarquia de prioridades

1. **Footprint** — RAM, binário, CPU. Menor possível. Inegociável.
2. **Segurança** — mTLS, token, hardening. Sempre on por default.
3. **DX** — Install em 1 comando, zero config, feedback imediato.
4. **Funcionalidade** — Métricas completas, comandos remotos, alertas.
5. **Scale** — TimescaleDB, Grafana, multi-user. Upgrade path, não requisito.

## O que NÃO fazemos

- Não adicionamos deps "pra facilitar" se custam binary size
- Não usamos `println!` — tracing sempre
- Não hardcodamos paths sem fallback
- Não quebramos backward compat de proto sem migration
- Não usamos unsafe sem justificativa documentada
- Não ignoramos clippy warnings
