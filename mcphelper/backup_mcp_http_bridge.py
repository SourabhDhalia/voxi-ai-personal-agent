#!/usr/bin/env python3
import json
import os
import sys
import urllib.request
import urllib.error

ENDPOINT = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("MCP_HTTP_URL", "").strip()
if not ENDPOINT:
    print("Missing MCP HTTP endpoint", file=sys.stderr)
    sys.exit(1)

TOKEN = (
    os.environ.get("MCP_ACCESS_TOKEN", "")
    or os.environ.get("MCP_BEARER_TOKEN", "")
    or os.environ.get("SWIGGY_MCP_TOKEN", "")
).strip()
if not TOKEN:
    token_file = os.environ.get("MCP_ACCESS_TOKEN_FILE", "").strip()
    if not token_file:
        token_file = os.environ.get(
            "SWIGGY_MCP_TOKEN_FILE",
            "/root/.tizenclaw/secrets/swiggy_food_token",
        )
    try:
        with open(token_file, "r", encoding="utf-8") as f:
            raw_token = f.read().strip()
            if raw_token.startswith("{"):
                parsed = json.loads(raw_token)
                TOKEN = (
                    parsed.get("access_token")
                    or parsed.get("token")
                    or parsed.get("bearer_token")
                    or ""
                ).strip()
            else:
                TOKEN = raw_token
    except FileNotFoundError:
        TOKEN = ""

SESSION_ID = None

def post_jsonrpc(payload: dict):
    global SESSION_ID

    body = json.dumps(payload).encode("utf-8")
    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json, text/event-stream",
        "MCP-Protocol-Version": "2025-11-25",
        "User-Agent": "tizenclaw-mcp-http-bridge/1.0",
    }

    if TOKEN:
        headers["Authorization"] = f"Bearer {TOKEN}"
    if SESSION_ID:
        headers["Mcp-Session-Id"] = SESSION_ID

    req = urllib.request.Request(
        ENDPOINT,
        data=body,
        headers=headers,
        method="POST",
    )

    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            new_session = resp.headers.get("Mcp-Session-Id") or resp.headers.get("MCP-Session-Id")
            if new_session:
                SESSION_ID = new_session

            raw = resp.read().decode("utf-8", errors="replace").strip()
            status = getattr(resp, "status", 200)

            if status == 202 or raw == "":
                return None

            # Basic JSON response
            if raw.startswith("{") or raw.startswith("["):
                return json.loads(raw)

            # Minimal SSE fallback: extract first data: JSON block
            if "data:" in raw:
                data_lines = []
                for line in raw.splitlines():
                    if line.startswith("data:"):
                        data_lines.append(line[5:].strip())
                merged = "\n".join([x for x in data_lines if x and x != "[DONE]"]).strip()
                if merged:
                    return json.loads(merged)

            raise RuntimeError(f"Unsupported response format: {raw[:500]}")
    except urllib.error.HTTPError as e:
        try:
            err_body = e.read().decode("utf-8", errors="replace")
        except Exception:
            err_body = ""
        err = {
            "jsonrpc": "2.0",
            "id": payload.get("id"),
            "error": {
                "code": -32000,
                "message": f"HTTP {e.code}: {err_body[:1000]}".strip()
            }
        }
        return err
    except Exception as e:
        err = {
            "jsonrpc": "2.0",
            "id": payload.get("id"),
            "error": {
                "code": -32001,
                "message": str(e)
            }
        }
        return err

def main():
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            msg = json.loads(line)
        except Exception as e:
            sys.stderr.write(f"Invalid JSON from stdin: {e}\n")
            sys.stderr.flush()
            continue

        resp = post_jsonrpc(msg)

        # JSON-RPC notifications often have no id; don't emit anything unless server gave us a response
        if resp is not None:
            sys.stdout.write(json.dumps(resp, separators=(",", ":")) + "\n")
            sys.stdout.flush()

if __name__ == "__main__":
    main()
