# HTTP Session API (v2)

This note documents the convenience HTTP endpoints backed by the app-server v2
thread/turn APIs. These endpoints are intended for API-key–scoped, stateless
session workflows in cluster deployments.

## Endpoints

### POST /session/start

Creates a new thread (session) using the same configuration logic as
`thread/start`.

**Request body**

```json
{
  "apiKeyId": "vsk_live_...",
  "model": "gpt-5",
  "modelProvider": "openai",
  "cwd": "/workspace",
  "approvalPolicy": "on-request",
  "sandbox": "workspace-write",
  "config": {}
}
```

**Response body**

```json
{
  "sessionId": "thread-123",
  "thread": { "id": "thread-123", "name": null, "summary": null, "createdAt": 0 },
  "model": "gpt-5",
  "modelProvider": "openai",
  "cwd": "/workspace",
  "approvalPolicy": "on-request",
  "sandbox": "workspace-write",
  "reasoningEffort": null
}
```

### POST /session/send

Submits a user turn to an existing session (thread). The server will emit the
`turn/started` notification and stream subsequent events on the SSE channel.

**Request body**

```json
{
  "apiKeyId": "vsk_live_...",
  "sessionId": "thread-123",
  "messages": [
    { "type": "text", "text": "Hello" }
  ]
}
```

**Response body**

```json
{
  "turnId": "turn-abc",
  "turn": { "id": "turn-abc", "items": [], "error": null, "status": "inProgress" }
}
```

### GET /session/events

Streams server-sent events associated with the session. This is the same event
stream surfaced by the app-server; the session parameters scope the output to a
single thread.

**Query params**

- `apiKeyId`
- `sessionId`

## Notes

- `apiKeyId` is accepted by the API surface to support downstream auth and
  routing logic in cluster deployments.
- These endpoints are convenience wrappers around the v2 `thread/*` and
  `turn/*` APIs; all session ids map directly to thread ids.
