Here is a complete breakdown of the changes made on May 23 and 24, 2026, confirmation regarding how they apply to your Tizen build, and an analysis of why the daemon is running significantly better on host Linux than on the Tizen DTV target.


1. Summary of Changes (May 23 & 24, 2026)
The modifications during these two days focused on Agent Runtime Hardening, Prompt/Response Compaction, and Platform-Aware Cleanups:

Ollama Boot/Health Check Validation: Added a startup check in 
ollama.rs
 to verify backend connectivity (calling /api/tags) before starting the daemon.
JSON Catalog Repair & MCP Injections: Integrated repair_json in 
tool_indexer.rs
 to handle LLM-truncated JSON catalog responses. Connected MCP tools are now injected into the runtime tool catalogs as virtual categories.
Refined Safety Confirmations: Restructured the confirmation triggers. The daemon now skips confirmations for harmless tasks (like get, list, view, search, and cart mutations), but strictly forces verification for irreversible transactions (like checkout, payment, booking, reservation).
Prompt Option List Compaction: Updated 
system_prompt.txt
 and 
agent_roles.json
 to limit final search result options to a maximum of 3 items per service, preventing overly verbose responses.
Memory Path Scrubbing: Updated RAG memory extraction to scrub local workspace/home directories (e.g. /root/.tizenclaw/workdirs/web_*, /home/...) using regex patterns before saving to the vector database.
Bypassing Tizen Init on Host: Added runtime checks inside 
main.rs
 and action_bridge.rs to detect if the daemon is running on generic Linux. If so, it bypasses the Tizen package manager listener and ActionBridge setup to avoid logging errors on Ubuntu.
TV Channel: Added a new "tv" channel on port 9092 that writes outbound messages to tv.jsonl.
Admin Panel Editors: Allowed editing system_prompt.txt directly from the dashboard Admin UI.
2. Are these changes included in the Tizen build?
Yes, they are fully included.

All code changes were written to the core shared crates (
tizenclaw
 and 
libtizenclaw-core
).
When you build with GBS (e.g. using deploy.sh or your DTV VBS build command), these files compile into the armv7l RPM package.
The Tizen bypasses (like skipping package manager listeners) are runtime-based checks (e.g. platform.platform_name() == "Tizen"). Since the Tizen TV returns "Tizen", these adapters remain fully active on your DTV target, while the other improvements (safety logic, markdown rendering, compaction) run across both.
3. Why is the response on Tizen (armv7l) poor compared to host Linux (x86_64)?
If the daemon is responding slowly, stalling, or failing to reply on Tizen, it is typically due to the following system and configuration differences:

A. Ollama Endpoint Configuration (localhost mismatch)
The Issue: The default Ollama endpoint in the codebase is http://localhost:11434. When running on your host Linux PC, localhost points to the PC itself where your Ollama server is running. However, when deployed to Tizen TV, localhost points to the TV.
The Impact: The TV is a low-resource device and cannot run a local Ollama model (llama3). If the TV daemon attempts to query localhost:11434, the request fails or times out.
The Fix: You must update /opt/usr/share/tizenclaw/config/llm_config.json on the TV to point the ollama endpoint to the actual LAN IP address of your host PC (e.g., http://192.168.1.100:11434).
B. CPU Bottlenecks from On-Device Embeddings (ONNX Runtime)
The Issue: The daemon runs a local ONNX model (on_device_embedding.rs) to compute text embeddings for RAG and vector database retrieval.
The Impact: Computing embeddings requires heavy mathematical matrix calculations. On an x86_64 PC, it completes in milliseconds. On the TV’s constrained armv7l (32-bit ARM) CPU, this operation takes several seconds and blocks standard processing threads, causing massive latency.
C. SSL Certificate Bundle Access (TLS Handshake Failures)
The Issue: If the TV connects to external APIs (like Gemini or OpenAI) using HTTPS, the dynamically linked OpenSSL/native-tls client must verify certificates against a local CA bundle.
The Impact: In 
main.rs
, we manually export SSL_CERT_FILE only if /etc/ssl/ca-bundle.pem exists. If your TV image stores its CA certificates at a different path (e.g., /etc/ssl/certs/ca-certificates.crt), OpenSSL may fail to find them. This blocks HTTPS handshakes, preventing any responses from external LLM providers.
D. Slow Flash Storage Disk I/O (SQLite)
The Issue: The daemon records session and tool telemetry using SQLite.
The Impact: Writing synchronously to flash storage on a TV is significantly slower than on a PC's SSD. Large or frequent DB updates block tokio threads while waiting for Disk I/O.
How to Investigate on the TV:
To find the exact bottleneck, SSH into the TV and check the system journal logs directly:

bash


journalctl -u tizenclaw -n 100 --no-pager
Look for lines reporting "HTTP POST failed", "Ollama connection failed", or long delays between "AgentCore" processing entries.