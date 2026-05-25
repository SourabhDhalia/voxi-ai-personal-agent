
Since you are building the RPM with GBS/VBS and installing it manually on the Tizen TV, you are bypassing the helper steps automated by deploy.sh.

To make your manual RPM installation work properly on Tizen DTV, you must configure these additional components manually:

1. Enable & Start the Tool Executor Socket
The RPM packages systemd unit files, but does not enable or start them automatically. If you do not activate the tool executor socket, the main daemon will fail to execute any terminal/system tools. Run these commands on the TV:

bash


systemctl daemon-reload
systemctl enable tizenclaw-tool-executor.socket
systemctl restart tizenclaw-tool-executor.socket
2. Always Run via Systemd, NEVER via tizenclaw &
If you launch the daemon from the terminal command line using tizenclaw &:

It runs under your current terminal user account (usually owner or developer).
It lacks the system capabilities (CAP_SYS_ADMIN, CAP_NET_ADMIN, etc.) and Tizen environment files.
Result: Native calls (DBus listeners, Tizen App Controls, and Package Manager APIs) will fail or log permissions/Cynara errors.
Always run it as the systemd service to ensure it runs as root with the correct capabilities:

bash


systemctl restart tizenclaw
3. Install the TizenClaw Web Bridge Widget (.wgt)
The RPM installs the backend services, but does not install the Tizen Web widget UI onto the TV. If you need the widget interface on the TV, copy and install the widget manually:

Copy the widget to /tmp/:
bash


scp data/wgt/TizenClawBridge.wgt root@<tv-ip>:/tmp/
Install it on the TV:
bash


ssh root@<tv-ip> 'pkgcmd -i -t wgt -p /tmp/TizenClawBridge.wgt -f'
4. Install ngrok for Tunnels (If Needed)
If you require external tunnels (e.g. for webhooks or dashboard access from outside the local network), you must copy ngrok manually to the TV since it is not bundled in the RPM:

Download the Linux ARM (32-bit) version of ngrok (since the TV is armv7l).
Copy it to /usr/bin/ngrok on the TV.
Make it executable:
bash


chmod +x /usr/bin/ngrok
5. Update the LLM Config file on the TV
Make sure you update the dynamic configuration file on the TV to point to your PC's IP address instead of localhost for Ollama.

File location: /opt/usr/share/tizenclaw/config/llm_config.json
Change the Ollama endpoint from "http://localhost:11434" to "http://<your-pc-ip>:11434".
11:48 PM



