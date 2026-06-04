# Alertas & Notificações

## Como funciona

O master monitora métricas de cada agent. Quando um threshold é ultrapassado por **3 leituras consecutivas**, dispara um alerta via webhook.

Quando o valor volta ao normal, o contador reseta.

## Configurar

```bash
sudo systemctl edit pulse-master
```

```ini
[Service]
Environment=PULSE_ALERT_WEBHOOK=https://api.telegram.org/bot<TOKEN>/sendMessage
Environment=PULSE_ALERT_TELEGRAM_CHAT_ID=123456789
Environment=PULSE_ALERT_CPU_PCT=90
Environment=PULSE_ALERT_MEM_PCT=90
Environment=PULSE_ALERT_BW_MBPS=100
```

```bash
sudo systemctl restart pulse-master
```

## Telegram

```bash
PULSE_ALERT_WEBHOOK=https://api.telegram.org/bot<BOT_TOKEN>/sendMessage
PULSE_ALERT_TELEGRAM_CHAT_ID=<CHAT_ID>
```

Mensagem enviada:
```
🚨 PULSE ALERT

Agent: `agent-0001`
Type: cpu_high
CPU at 95.2% (threshold: 90%)
```

## Slack

```bash
PULSE_ALERT_WEBHOOK=https://hooks.slack.com/services/T.../B.../xxx
```

## Discord

```bash
PULSE_ALERT_WEBHOOK=https://discord.com/api/webhooks/<ID>/<TOKEN>
```

## Webhook genérico

Qualquer outra URL recebe JSON:
```json
{
  "agent": "agent-0001",
  "type": "cpu_high",
  "message": "CPU at 95.2% (threshold: 90%)",
  "timestamp": "1717455600"
}
```

## Tipos de alerta

| Tipo | Trigger |
|---|---|
| `cpu_high` | CPU > threshold por 3 leituras |
| `mem_high` | Memory > threshold por 3 leituras |
| `bandwidth_high` | Network > threshold por 3 leituras |
| `agent_offline` | Sem métricas por >45 segundos |
