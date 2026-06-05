# Alerting

## How It Works

The master monitors metrics from each agent. When a threshold is exceeded for **3 consecutive readings**, an alert fires via webhook. When the value returns to normal, the counter resets.

## Configuration

```bash
sudo systemctl edit pulse-master
```

```ini
[Service]
Environment=PULSE_ALERT_WEBHOOK=https://api.telegram.org/bot<BOT_TOKEN>/sendMessage
Environment=PULSE_ALERT_TELEGRAM_CHAT_ID=123456789
Environment=PULSE_ALERT_CPU_PCT=90
Environment=PULSE_ALERT_MEM_PCT=90
Environment=PULSE_ALERT_BW_MBPS=100
```

```bash
sudo systemctl restart pulse-master
```

## Alert Types

| Type | Trigger | Config Variable |
|------|---------|-----------------|
| `cpu_high` | CPU > threshold for 3 consecutive readings | `PULSE_ALERT_CPU_PCT` (default: 90) |
| `mem_high` | Memory > threshold for 3 consecutive readings | `PULSE_ALERT_MEM_PCT` (default: 90) |
| `bandwidth_high` | Network > threshold for 3 consecutive readings | `PULSE_ALERT_BW_MBPS` (default: 100) |
| `agent_offline` | No metrics received for > 45 seconds | Automatic |

## Webhook Channels

### Telegram

```bash
PULSE_ALERT_WEBHOOK=https://api.telegram.org/bot<BOT_TOKEN>/sendMessage
PULSE_ALERT_TELEGRAM_CHAT_ID=<CHAT_ID>
```

### Slack

```bash
PULSE_ALERT_WEBHOOK=https://hooks.slack.com/services/T.../B.../xxx
```

### Discord

```bash
PULSE_ALERT_WEBHOOK=https://discord.com/api/webhooks/<ID>/<TOKEN>
```

### Generic Webhook

Any other URL receives a JSON payload:

```json
{
  "agent": "agent-0001",
  "type": "cpu_high",
  "message": "CPU at 95.2% (threshold: 90%)",
  "timestamp": "1717455600"
}
```

## Channel Auto-Detection

The master detects the webhook type from the URL:

| URL contains | Detected as |
|--------------|-------------|
| `api.telegram.org` | Telegram (requires `PULSE_ALERT_TELEGRAM_CHAT_ID`) |
| `hooks.slack.com` | Slack |
| `discord.com/api/webhooks` | Discord |
| Anything else | Generic JSON POST |

## Viewing Alert History

```bash
curl -s http://master:8080/api/alerts | jq '.alerts[]'
```

Or via the TUI Fleet view (alerts appear as status indicators).
