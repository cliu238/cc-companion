# Anthropic OAuth Usage API Reference

## Endpoint

```
GET https://api.anthropic.com/api/oauth/usage
```

## Required Headers

| Header | Value |
|---|---|
| Accept | `application/json, text/plain, */*` |
| Content-Type | `application/json` |
| User-Agent | `claude-code/2.0.32` |
| Authorization | `Bearer <access_token>` |
| anthropic-beta | `oauth-2025-04-20` |

The `anthropic-beta` header is required; requests without it will fail.

## Authentication

Claude Code stores OAuth credentials in macOS Keychain under the service name `Claude Code-credentials`.

### Retrieve credentials (macOS)

```bash
security find-generic-password -s "Claude Code-credentials" -w
```

Returns a JSON object:

```json
{
  "claudeAiOauth": {
    "accessToken": "sk-ant-oat01-...",
    "refreshToken": "...",
    "expiresAt": 1234567890,
    "scopes": ["..."],
    "subscriptionType": "pro"
  }
}
```

The `accessToken` field is the Bearer token for API requests.

### Linux / other platforms

Credentials may be stored differently (e.g., `~/.claude/credentials.json` or via a secret service). Check Claude Code's documentation for the credential storage path on the target platform.

## Response Schema

```json
{
  "five_hour": {
    "utilization": 6.0,
    "resets_at": "2025-11-04T04:59:59.943648+00:00"
  },
  "seven_day": {
    "utilization": 35.0,
    "resets_at": "2025-11-06T03:59:59.943679+00:00"
  },
  "seven_day_oauth_apps": null,
  "seven_day_opus": {
    "utilization": 0.0,
    "resets_at": null
  },
  "iguana_necktie": null
}
```

### Fields

| Field | Type | Description |
|---|---|---|
| `five_hour` | object or null | 5-hour rolling window usage |
| `five_hour.utilization` | number | Percentage used (0-100) |
| `five_hour.resets_at` | string (ISO 8601) or null | When the window resets |
| `seven_day` | object or null | 7-day rolling window usage |
| `seven_day.utilization` | number | Percentage used (0-100) |
| `seven_day.resets_at` | string (ISO 8601) or null | When the window resets |
| `seven_day_opus` | object or null | Opus-specific 7-day usage (if applicable) |
| `seven_day_oauth_apps` | null | Reserved for OAuth app-specific limits |

### TypeScript Types

```typescript
interface UsageLimits {
  five_hour: {
    utilization: number;
    resets_at: string | null;
  } | null;
  seven_day: {
    utilization: number;
    resets_at: string | null;
  } | null;
  seven_day_opus?: {
    utilization: number;
    resets_at: string | null;
  } | null;
}

interface Credentials {
  claudeAiOauth: {
    accessToken: string;
    refreshToken: string;
    expiresAt: number;
    scopes: string[];
    subscriptionType: string;
  };
}
```

## Error Handling

| Scenario | Behavior |
|---|---|
| Invalid/expired token | HTTP 401 — re-authenticate with `claude auth login` |
| Missing `anthropic-beta` header | HTTP 400 or 403 |
| No active subscription | `five_hour` and `seven_day` may be null |
| Network error | Connection failure — check internet connectivity |

## Notes

- This is an undocumented internal API; Anthropic may change it without notice
- The `utilization` values match what `/usage` displays inside Claude Code
- Token expiry is in `expiresAt` (Unix epoch seconds); refresh if expired
- The `User-Agent` version string should approximate the current Claude Code version
