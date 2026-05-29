---
id: mcp_sqlite_reload
name: SQLite Reload Workflow
description: Check SQLite status and reload schema
trigger: manual
---

## Step 1: Run SQLite check query
- type: tool
- tool_name: mcp_sqlite_query
- args: {"sql": "SELECT 1;"}
- output_var: query_res
- skip_on_failure: true
