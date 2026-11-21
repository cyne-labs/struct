#!/usr/bin/env python3
"""
Server launcher for Struct Milestone R4: retrieval accuracy across formats.

Follows RULES.md Rule 7 and R4.md:
- Model: Gemma 4 E2B (gemma-4-E2B-it-Q4_K_M.gguf), per R4 decision 14
- Binary: C:\\Users\\Ashman Das\\Documents\\turboquant-llama-cpp\\llama-server.exe
- Exact flags:
  -ngl 999 --mlock --no-mmap --cache-type-k turbo4 --cache-type-v turbo3 -fa on
  -c 65536 -np 4 --kv-unified --reasoning-budget 8192 --temp 1.0 --top-p 0.95 --top-k 64 --min-p 0.0
  --presence-penalty 0.0 --repeat-penalty 1.0 --port 8090 --host 127.0.0.1
- Writes exact command line to bench/r4/real/launch_line.txt (hashed in SHA256SUMS).
- Has a stop command.
"""

import os
import sys
import time
import subprocess
import urllib.request
import urllib.error

BENCH_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
R4_DIR = os.path.join(BENCH_DIR, "r4")
# R4_REAL_DIR (optional) sends a repeat run to its own folder, e.g. real_run2, so it never overwrites real/.
REAL_DIR = os.environ.get("R4_REAL_DIR") or os.path.join(R4_DIR, "real")
LAUNCH_LINE_FILE = os.path.join(REAL_DIR, "launch_line.txt")
PID_FILE = os.path.join(REAL_DIR, "llama_server.pid")
SHA256_FILE = os.path.join(REAL_DIR, "SHA256SUMS")

SERVER_DIR = r"C:\Users\Ashman Das\Documents\turboquant-llama-cpp"
LLAMA_SERVER = os.path.join(SERVER_DIR, "llama-server.exe")
# R4_PROFILE picks the model. Default "gemma4e2b" (R4 decision 14); "qwen35_4b" is Qwen3.5-4B with MTP.
R4_PROFILE = os.environ.get("R4_PROFILE", "gemma4e2b")
PROFILES = {
    "gemma4e2b": {
        "model": os.path.join(SERVER_DIR, "models", "lmstudio-community", "gemma-4-E2B-it-GGUF", "gemma-4-E2B-it-Q4_K_M.gguf"),
        "stem": "gemma4e2b",
        "sampling": {"temperature": 1.0, "top_p": 0.95, "top_k": 64, "min_p": 0.0, "presence_penalty": 0.0, "repeat_penalty": 1.0},
        "extra_flags": [],
    },
    "qwen35_4b": {
        "model": os.path.join(SERVER_DIR, "models", "unsloth", "Qwen3.5-4B-GGUF", "Qwen3.5-4B-Q4_K_M.gguf"),
        "stem": "qwen35_4b",
        # Qwen3.5 model card, thinking mode, general tasks
        "sampling": {"temperature": 1.0, "top_p": 0.95, "top_k": 20, "min_p": 0.0, "presence_penalty": 1.5, "repeat_penalty": 1.0},
        "extra_flags": ["--spec-type", "draft-mtp,ngram-mod", "--spec-draft-n-max", "3", "-ctkd", "turbo3", "-ctvd", "turbo2",
                        "--spec-ngram-mod-n-max", "3", "--spec-ngram-mod-n-min", "2", "--spec-ngram-mod-n-match", "8"],
    },
}
PROFILE = PROFILES[R4_PROFILE]
MODEL_PATH = PROFILE["model"]

HOST = "127.0.0.1"
PORT = 8090
SERVER_BASE = f"http://{HOST}:{PORT}"

# Exact R4 flags per RULES rule 7, R4.md, and milestone specification
R4_FLAGS = [
    "-ngl", "999",
    "--mlock",
    "--no-mmap",
    "--cache-type-k", "turbo4",
    "--cache-type-v", "turbo3",
    "-fa", "on",
    "-c", "65536",
    "-np", "4",
    "--kv-unified",
    "--reasoning-budget", "8192",
    "--temp", str(PROFILE["sampling"]["temperature"]),
    "--top-p", str(PROFILE["sampling"]["top_p"]),
    "--top-k", str(PROFILE["sampling"]["top_k"]),
    "--min-p", str(PROFILE["sampling"]["min_p"]),
    "--presence-penalty", str(PROFILE["sampling"]["presence_penalty"]),
    "--repeat-penalty", str(PROFILE["sampling"]["repeat_penalty"]),
    "--port", str(PORT),
    "--host", HOST,
] + list(PROFILE["extra_flags"])

def get_launch_cmd_args():
    """Returns the full argument list for starting llama-server."""
    return [LLAMA_SERVER, "-m", MODEL_PATH] + list(R4_FLAGS)

def format_cmd_line(args):
    """Formats an argument list into a clean command-line string with proper quoting for paths with spaces."""
    parts = []
    for arg in args:
        if " " in arg and not (arg.startswith('"') and arg.endswith('"')):
            parts.append(f'"{arg}"')
        else:
            parts.append(arg)
    return " ".join(parts)

def get_exact_launch_line():
    """Returns the exact launch line string."""
    return format_cmd_line(get_launch_cmd_args())

def update_sha256sums():
    """Updates SHA256SUMS in bench/r4/real/."""
    # Import update_sha256sums from collect to keep single source of truth
    sys.path.insert(0, BENCH_DIR)
    from r4.collect import update_sha256sums as coll_update_sha
    coll_update_sha()

def write_launch_line(output_path=LAUNCH_LINE_FILE, update_sha=True):
    """
    Writes the exact command line to launch_line.txt and updates SHA256SUMS.
    """
    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    line = get_exact_launch_line()
    with open(output_path, "w", encoding="utf-8", newline="\n") as f:
        f.write(line + "\n")
    if update_sha:
        update_sha256sums()
    return line

def check_gpu_rule7():
    """
    RULES Rule 7: check nvidia-smi total used memory before starting server.
    Reads GPU's total used memory and stops if > 1 GB is in use.
    If even that cannot be read, stops with a clear message rather than passing silently.
    Returns True if GPU memory <= 1024 MB, False otherwise.
    """
    try:
        res = subprocess.check_output(
            ["nvidia-smi", "--query-gpu=memory.used", "--format=csv,noheader,nounits"],
            text=True
        )
        lines = [l.strip() for l in res.strip().splitlines() if l.strip()]
        if not lines:
            print("ERROR: nvidia-smi returned empty output when querying GPU memory.used")
            return False
        used_mb = int(lines[0])
        if used_mb > 1024:
            print(f"ERROR: GPU memory already in use: {used_mb} MB (> 1024 MB limit). RULES Rule 7 violated.")
            return False
        print(f"GPU memory check passed: {used_mb} MB currently in use (<= 1024 MB).")
        return True
    except Exception as e:
        print(f"ERROR: Could not query or parse nvidia-smi GPU memory.used: {e}")
        return False

def start_server(wait_ready=True, timeout=120):
    """
    Starts the turboquant llama-server with exactly the R4 flags.
    Writes exact launch line to launch_line.txt and updates SHA256SUMS.
    """
    if not check_gpu_rule7():
        print("ERROR: GPU is currently busy (>1GB in use) or nvidia-smi check failed. Aborting start.")
        sys.exit(1)

    # Write launch line before starting
    launch_line = write_launch_line()
    print(f"Recorded launch line to {LAUNCH_LINE_FILE}:\n  {launch_line}\n")

    cmd = get_launch_cmd_args()
    print("Starting llama-server...")
    proc = subprocess.Popen(
        cmd,
        cwd=SERVER_DIR,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL
    )

    with open(PID_FILE, "w", encoding="utf-8") as f:
        f.write(str(proc.pid))

    if not wait_ready:
        print(f"Started llama-server with PID {proc.pid}")
        return proc

    ready = False
    print(f"Waiting for server at {SERVER_BASE}/health (timeout {timeout}s)...")
    for i in range(timeout):
        time.sleep(1)
        if proc.poll() is not None:
            raise RuntimeError(f"Server process exited prematurely with code {proc.returncode}")
        try:
            with urllib.request.urlopen(f"{SERVER_BASE}/health", timeout=2) as res:
                if res.getcode() == 200:
                    ready = True
                    print(f"Server ready after {i+1}s.")
                    break
        except Exception:
            pass

    if not ready:
        stop_server(proc)
        raise RuntimeError(f"Server failed to become healthy within {timeout}s")

    return proc

def stop_server(proc=None):
    """
    Stops the llama-server process.
    Ensures no lingering llama-server processes per Rule 7.
    """
    print("Stopping llama-server...")
    if proc:
        try:
            proc.terminate()
            proc.wait(timeout=15)
        except Exception:
            try:
                proc.kill()
            except Exception:
                pass
    elif os.path.exists(PID_FILE):
        try:
            with open(PID_FILE, "r", encoding="utf-8") as f:
                pid = int(f.read().strip())
            subprocess.run(["taskkill", "/F", "/PID", str(pid)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        except Exception:
            pass

    # Clean up pidfile
    if os.path.exists(PID_FILE):
        try:
            os.remove(PID_FILE)
        except Exception:
            pass

    # Ensure no lingering llama-server processes per Rule 7 and R3 precedent
    subprocess.run(["taskkill", "/F", "/IM", "llama-server.exe"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(1)
    print("Server stopped.")

def main():
    if len(sys.argv) < 2:
        print("Usage: python launch_server.py [start|stop|write-line|status]")
        print("  start      Start the turboquant llama-server with R4 flags")
        print("  stop       Stop the running llama-server")
        print("  write-line Write launch_line.txt and update SHA256SUMS without starting")
        print("  status     Check server health at 127.0.0.1:8090")
        sys.exit(0)

    cmd = sys.argv[1].lower()
    if cmd == "start":
        start_server()
    elif cmd == "stop":
        stop_server()
    elif cmd in ("write-line", "write_line", "record"):
        line = write_launch_line()
        print(f"Wrote launch line to {LAUNCH_LINE_FILE}:\n{line}")
    elif cmd == "status":
        try:
            with urllib.request.urlopen(f"{SERVER_BASE}/health", timeout=5) as res:
                print(f"Health: {res.read().decode('utf-8')}")
        except Exception as e:
            print(f"Server not reachable: {e}")
    else:
        print(f"Unknown command: {cmd}")
        sys.exit(1)

if __name__ == "__main__":
    main()
