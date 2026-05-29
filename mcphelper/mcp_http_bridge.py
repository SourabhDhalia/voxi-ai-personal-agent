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

# Optional initial MCP session id:
# args: URL --session SESSION_ID
# args: URL --mcp-session-id SESSION_ID
# env : MCP_SESSION_ID=SESSION_ID
SESSION_ID = os.environ.get("MCP_SESSION_ID", "").strip()

args = sys.argv[2:]
for i, arg in enumerate(args):
    if arg in ("--session", "--mcp-session-id", "--mcp-session", "mcp-session-id"):
        if i + 1 < len(args):
            SESSION_ID = args[i + 1].strip()

TOKEN = (
    os.environ.get("MCP_ACCESS_TOKEN", "")
    or os.environ.get("MCP_BEARER_TOKEN", "")
    or os.environ.get("SWIGGY_MCP_TOKEN", "")
    or os.environ.get("ZEPTO_MCP_TOKEN", "")
).strip()

if not TOKEN:
    token_file = os.environ.get("MCP_ACCESS_TOKEN_FILE", "").strip()
    if not token_file:
        token_file = (
            os.environ.get("SWIGGY_MCP_TOKEN_FILE", "").strip()
            or os.environ.get("ZEPTO_MCP_TOKEN_FILE", "").strip()
            or "/root/.voxi/secrets/swiggy_food_token"
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

def log(msg: str):
    sys.stderr.write(msg + "\n")
    sys.stderr.flush()

def get_session_header(headers):
    # urllib headers are case-insensitive in normal use, but this is safer.
    for key in headers.keys():
        if key.lower() == "mcp-session-id":
            value = headers.get(key)
            if value:
                return value.strip()
    return None

def parse_response_body(raw: str):
    raw = raw.strip()
    if not raw:
        return None

    # Normal JSON response
    if raw.startswith("{") or raw.startswith("["):
        return json.loads(raw)

    # SSE response: collect data: lines.
    if "data:" in raw:
        data_lines = []
        for line in raw.splitlines():
            line = line.strip()
            if line.startswith("data:"):
                data = line[5:].strip()
                if data and data != "[DONE]":
                    data_lines.append(data)

        merged = "\n".join(data_lines).strip()
        if merged:
            return json.loads(merged)

    raise RuntimeError(f"Unsupported response format: {raw[:500]}")

def post_jsonrpc(payload: dict):
    global SESSION_ID

    method = payload.get("method", "")
    body = json.dumps(payload).encode("utf-8")

    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json, text/event-stream",
        "MCP-Protocol-Version": "2025-11-25",
        "User-Agent": "voxi-mcp-http-bridge/1.0",
    }

    if TOKEN:
        headers["Authorization"] = f"Bearer {TOKEN}"

    if SESSION_ID:
        headers["Mcp-Session-Id"] = SESSION_ID

    log(
        f"MCP HTTP bridge request method={method or 'unknown'} "
        f"session={'yes' if SESSION_ID else 'no'} "
        f"token={'yes' if TOKEN else 'no'} "
        f"endpoint={ENDPOINT}"
    )

    req = urllib.request.Request(
        ENDPOINT,
        data=body,
        headers=headers,
        method="POST",
    )

    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            new_session = get_session_header(resp.headers)
            if new_session:
                SESSION_ID = new_session
                log(f"MCP session updated: {SESSION_ID}")

            raw = resp.read().decode("utf-8", errors="replace").strip()
            status = getattr(resp, "status", 200)

            if status == 202 or raw == "":
                return None

            return parse_response_body(raw)

    except urllib.error.HTTPError as e:
        new_session = get_session_header(e.headers)
        if new_session:
            SESSION_ID = new_session
            log(f"MCP session updated from error response: {SESSION_ID}")

        try:
            err_body = e.read().decode("utf-8", errors="replace")
        except Exception:
            err_body = ""

        return {
            "jsonrpc": "2.0",
            "id": payload.get("id"),
            "error": {
                "code": -32000,
                "message": f"HTTP {e.code}: {err_body[:1000]}".strip()
            }
        }

    except Exception as e:
        return {
            "jsonrpc": "2.0",
            "id": payload.get("id"),
            "error": {
                "code": -32001,
                "message": str(e)
            }
        }

def main():
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            msg = json.loads(line)
        except Exception as e:
            log(f"Invalid JSON from stdin: {e}")
            continue

        resp = post_jsonrpc(msg)

        # JSON-RPC notifications often have no id.
        # Do not emit anything unless server gave us a response.
        if resp is not None:
            sys.stdout.write(json.dumps(resp, separators=(",", ":")) + "\n")
            sys.stdout.flush()

if __name__ == "__main__":
    main()
