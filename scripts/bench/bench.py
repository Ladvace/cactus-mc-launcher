"""Compare launcher builds on macOS: startup to first window, memory, CPU at rest.

Each run gets a fresh fake home (HOME and CFFIXED_USER_HOME, so WebKit's storage moves too)
holding only `settings.json` with the scenario's background, so your real data is never
touched. Memory and CPU count the app's process tree plus any WebKit XPC processes
(WebContent, GPU, Networking) that appear after launch, since those are the webview's and
aren't children of the app. Each window is brought to the front for its sample and a run only
counts if it stayed there, so leave the machine alone while this runs (about 13 minutes).

    swiftc -O scripts/bench/winwait.swift -o scripts/bench/winwait
    swiftc -O scripts/bench/isfront.swift -o scripts/bench/isfront
    cargo build --release
    python3 -I scripts/bench/bench.py results.json rune=target/release/cactus-launcher \\
        tauri=/path/to/tauri/build/cactus-launcher

The Tauri build reads its onboarding flag from localStorage, which can't be preset; to measure
it past onboarding, default `cactus:onboarded` to true in its `+layout.svelte` before building.
"""
import json, os, re, shutil, signal, subprocess, sys, time

HERE = os.path.dirname(os.path.abspath(__file__))
WINWAIT = os.path.join(HERE, "winwait")
ISFRONT = os.path.join(HERE, "isfront")
APP_DIR = "Library/Application Support/com.ladvace.cactuslauncher"
SCENARIOS = {"plain": "", "beams": "anim:beams", "nebula": "anim:nebula"}
RUNS = 3
SETTLE = 12      # seconds after the window appears before measuring
SAMPLE = 20      # seconds of CPU sampling


def ps(fields, pids):
    out = subprocess.run(["ps", "-o", fields, "-p", ",".join(map(str, pids))], capture_output=True, text=True).stdout
    lines = out.strip().splitlines()
    # `-o name=` prints no header line; a field without `=` does.
    if not all(field.endswith("=") for field in fields.split(",")):
        lines = lines[1:]
    return [line.split(None, len(fields.split(",")) - 1) for line in lines]


def webkit_pids():
    out = subprocess.run(["pgrep", "-f", "com.apple.WebKit"], capture_output=True, text=True).stdout
    return {int(p) for p in out.split()}


def descendants(pid):
    found, frontier = set(), [pid]
    while frontier:
        out = subprocess.run(["pgrep", "-P", str(frontier.pop())], capture_output=True, text=True).stdout
        for child in map(int, out.split()):
            if child not in found:
                found.add(child)
                frontier.append(child)
    return found


def bring_to_front(pid):
    """A process started from a script opens behind the active app, and a hidden window may
    stop drawing (WebKit throttles, Rune skips frames), so put it in front for the sample."""
    subprocess.run(["osascript", "-e", f'tell application "System Events" to set frontmost of (first process whose unix id is {pid}) to true'],
                   capture_output=True)


def is_front(pid):
    return subprocess.run([ISFRONT, str(pid)], capture_output=True, text=True).stdout.strip() == "front"


def cpu_seconds(pids):
    total = 0.0
    for (t,) in ps("time=", pids):
        parts = t.strip().split(":")
        secs = float(parts[-1]) + 60 * float(parts[-2]) + (3600 * float(parts[-3]) if len(parts) > 2 else 0)
        total += secs
    return total


def footprint_mb(pids):
    """Physical footprint (what Activity Monitor's Memory column shows), summed."""
    total = 0.0
    for pid in pids:
        out = subprocess.run(["footprint", "-p", str(pid)], capture_output=True, text=True).stdout
        m = re.search(r"Footprint:\s*([\d.]+)\s*([KMG]B)", out)
        if m:
            total += float(m.group(1)) * {"KB": 1 / 1024, "MB": 1, "GB": 1024}[m.group(2)]
    return total


def run_once(binary, background):
    home = os.path.join(HERE, "home")
    shutil.rmtree(home, ignore_errors=True)
    os.makedirs(os.path.join(home, APP_DIR))
    with open(os.path.join(home, APP_DIR, "settings.json"), "w") as f:
        json.dump({"background": background}, f)
    # Past onboarding, so the background is what's on screen (the Tauri build under test reads
    # this flag from localStorage; its benchmark build defaults it to true instead).
    with open(os.path.join(home, APP_DIR, "ui-prefs.json"), "w") as f:
        json.dump({"cactus:onboarded": True}, f)
    env = dict(os.environ, HOME=home, CFFIXED_USER_HOME=home, RUNE_PERF_WARN="0", RUNE_A11Y_WARN="0")
    before = webkit_pids()
    proc = subprocess.Popen([binary], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        waited = subprocess.run([WINWAIT, str(proc.pid), "60"], capture_output=True, text=True).stdout.strip()
        startup = None if waited == "timeout" else float(waited)
        bring_to_front(proc.pid)
        time.sleep(SETTLE)
        pids = {proc.pid} | descendants(proc.pid) | (webkit_pids() - before)
        memory = footprint_mb(pids)
        front_before = is_front(proc.pid)
        c0, t0 = cpu_seconds(pids), time.time()
        time.sleep(SAMPLE)
        cpu = (cpu_seconds(pids) - c0) / (time.time() - t0) * 100
        # Only a window that stayed in front for the whole sample counts (see `summary`).
        visible = front_before and is_front(proc.pid)
        return {"startup_s": startup, "memory_mb": round(memory, 1), "cpu_pct": round(cpu, 1),
                "visible": visible, "processes": len(pids)}
    finally:
        proc.send_signal(signal.SIGTERM)
        try:
            proc.wait(10)
        except subprocess.TimeoutExpired:
            proc.kill()
        time.sleep(3)  # let WebKit's XPC processes exit before the next run counts them


def main():
    out_path, apps = sys.argv[1], dict(arg.split("=", 1) for arg in sys.argv[2:])
    results = {}
    for name, binary in apps.items():
        results[name] = {"binary_mb": round(os.path.getsize(binary) / 2**20, 1)}
        for scenario, background in SCENARIOS.items():
            runs = []
            for i in range(RUNS):
                r = run_once(binary, background)
                print(name, scenario, i, r, flush=True)
                runs.append(r)
            results[name][scenario] = runs
    with open(out_path, "w") as f:
        json.dump(results, f, indent=1)


if __name__ == "__main__":
    main()
