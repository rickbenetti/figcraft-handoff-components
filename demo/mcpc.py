"""Minimal MCP stdio client for `figcraft-cli mcp` (one headless session kept alive)."""
import json, subprocess, itertools
class FC:
    def __init__(self):
        self.p = subprocess.Popen(["figcraft-cli", "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, bufsize=1)
        self.n = itertools.count(1)
        self.rpc("initialize", {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "demo-ds", "version": "1"}})
        self.send({"jsonrpc": "2.0", "method": "notifications/initialized"})
    def send(self, m): self.p.stdin.write(json.dumps(m) + "\n"); self.p.stdin.flush()
    def rpc(self, method, params):
        i = next(self.n); self.send({"jsonrpc": "2.0", "id": i, "method": method, "params": params})
        while True:
            l = self.p.stdout.readline()
            if not l: raise SystemExit("mcp stream ended")
            m = json.loads(l)
            if m.get("id") == i: return m
    def tool(self, name, args):
        r = self.rpc("tools/call", {"name": name, "arguments": args})
        if "error" in r: raise RuntimeError(r["error"])
        res = r["result"]
        txt = "".join(c.get("text", "") for c in res.get("content", []) if c.get("type") == "text")
        if res.get("isError"): raise RuntimeError(txt[:600])
        try: return json.loads(txt)
        except Exception: return txt
    def cmd(self, command, **params):
        return self.tool("run_command", {"command": command, "params": params})
