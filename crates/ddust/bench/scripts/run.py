"""Runs the benches into a directory a pull request commits: two passes, each a process of its own,
on one core, with a manifest of the machine, the toolchain and the contenders.

Usage: run.py --pr <n> --what <what> --host <host> [--passes <n>] [--purpose <text>] [--core <cpu>]
              [--filter <word>]...

The run is `results/<UTC time>-pr<n>-<what>-<host>/`: `ops-1.txt`, `ops-2.txt` and so on, the table
each pass printed; `ops-1.toml` and the rest, every figure; the same for `mixed`; `code.txt`, each
probe's instructions; and `manifest.toml`. A figure whose passes disagree, by more than 2% in time
or cycles or 0.5% in instructions, is named at the end: such a run is taken again with five passes,
and the book cites the median of their medians, with their range.

On Linux each pass runs under `taskset` on `--core`, an isolated CPU. macOS pins nothing: run on mains
power with nothing else open, and as root for the counters, built with `--features kperf`.
"""

import argparse
import datetime
import json
import os
import pathlib
import platform
import shutil
import subprocess
import sys
import tomllib

HERE = pathlib.Path(__file__).resolve().parent.parent
BENCHES = ("ops", "mixed")
# How far two passes' medians may differ, as a share: time and cycles, and instructions.
AGREEMENT = {"nanoseconds": 0.02, "cycles": 0.02, "instructions": 0.005}


def output(*command):
    """What `command` prints, or "" where it fails."""
    try:
        return subprocess.run(command, capture_output=True, text=True, check=True).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return ""


def read(path):
    """A file's text, or "" where it cannot be read."""
    try:
        return pathlib.Path(path).read_text().strip()
    except OSError:
        return ""


def build(features):
    """Builds every bench and the probe, and the path of each bench's binary."""
    command = ["cargo", "bench", "--locked", "--no-run", "--message-format=json", *features]
    out = subprocess.run(command, cwd=HERE, capture_output=True, text=True, check=True).stdout
    binaries = {}
    for line in out.splitlines():
        message = json.loads(line)
        target = message.get("target", {})
        if message.get("reason") == "compiler-artifact" and "bench" in target.get("kind", []):
            binaries[target["name"]] = message["executable"]
    subprocess.run(["cargo", "build", "--locked", "--profile", "bench", "--example", "probe", *features],
                   cwd=HERE, check=True)
    return binaries


def placement(core):
    """The command prefix that pins a pass, and how the manifest names it."""
    if platform.system() == "Linux" and shutil.which("taskset"):
        return ["taskset", "-c", str(core)], f"taskset -c {core}"
    return [], "none: this platform pins no thread"


def toml_string(text):
    """`text` as a TOML basic string."""
    return json.dumps(text, ensure_ascii=False)


# Arm's part numbers, from a core's MIDR, for the cores a server or a board runs.
ARM_PARTS = {
    0xD0C: "Neoverse N1", 0xD40: "Neoverse V1", 0xD49: "Neoverse N2", 0xD4F: "Neoverse V2",
    0xD8E: "Neoverse N3", 0xD84: "Neoverse V3", 0xD0B: "Cortex-A76", 0xD41: "Cortex-A78",
}


def hardware(core):
    """The CPU, the machine, its cores and the caches `core` uses, as the platform reports them."""
    if platform.system() == "Darwin":
        cpu = output("sysctl", "-n", "machdep.cpu.brand_string")
        caches = ", ".join(
            f"{name} {output('sysctl', '-n', f'hw.perflevel0.{key}')} bytes"
            for name, key in (("L1i", "l1icachesize"), ("L1d", "l1dcachesize"), ("L2", "l2cachesize"))
        )
        return cpu, output("sysctl", "-n", "hw.model"), output("sysctl", "-n", "hw.ncpu"), caches
    lscpu = {}
    for line in output("lscpu").splitlines():
        key, _, value = line.partition(":")
        lscpu.setdefault(key.strip(), value.strip())
    cpu = lscpu.get("Model name", "")
    midr = read(f"/sys/devices/system/cpu/cpu{core}/regs/identification/midr_el1")
    if midr:
        part = (int(midr, 16) >> 4) & 0xFFF
        cpu = f"{ARM_PARTS.get(part, f'Arm part {part:#x}')}, MIDR {midr}"
    caches = []
    for index in sorted(pathlib.Path(f"/sys/devices/system/cpu/cpu{core}/cache").glob("index*")):
        level, kind, size = (read(index / name) for name in ("level", "type", "size"))
        caches.append(f"L{level} {kind.lower()} {size}")
    machine = read("/sys/devices/virtual/dmi/id/product_name")
    return cpu, machine, lscpu.get("CPU(s)", ""), ", ".join(caches)


def contenders():
    """Each contender's version, from the bench's lock."""
    lock = tomllib.loads((HERE / "Cargo.lock").read_text())
    wanted = {"bigdecimal", "decimal-rs", "fastnum", "fin_decimal", "fixdec", "fixed", "fixnum", "nexus-decimal",
              "primitive_fixed_point_decimal", "rust_decimal", "perf-event2", "darwin-kperf"}
    return {package["name"]: package["version"] for package in lock["package"] if package["name"] in wanted}


def manifest(run_id, purpose, placed, core, features, filters, passes):
    """The run's manifest, as TOML."""
    cpu, machine, cores, caches = hardware(core)
    linux = platform.system() == "Linux"
    lines = [
        f"run-id  = {toml_string(run_id)}",
        f"purpose = {toml_string(purpose)}",
        "",
        "[hardware]",
        f"cpu     = {toml_string(cpu)}",
        f"machine = {toml_string(machine)}",
        f"cores   = {toml_string(cores)}",
        f"caches  = {toml_string(caches)}",
        "",
        "[environment]",
        f"system    = {toml_string(platform.platform())}",
        f"placement = {toml_string(placed)}",
    ]
    if linux:
        lines += [
            f"isolated-cpus       = {toml_string(read('/sys/devices/system/cpu/isolated'))}",
            f"perf-event-paranoid = {toml_string(read('/proc/sys/kernel/perf_event_paranoid'))}",
            f"aslr                = {toml_string(read('/proc/sys/kernel/randomize_va_space'))}",
            f"transparent-hugepages = {toml_string(read('/sys/kernel/mm/transparent_hugepage/enabled'))}",
            f"governor            = {toml_string(read(f'/sys/devices/system/cpu/cpu{core}/cpufreq/scaling_governor') or 'none visible')}",
        ]
    rustc = output("rustc", "-vV").splitlines()
    lines += [
        "",
        "[toolchain]",
        f"rustc    = {toml_string(rustc[0] if rustc else '')}",
        f"llvm     = {toml_string(next((line for line in rustc if line.startswith('LLVM')), ''))}",
        f"profile  = {toml_string('bench: release with fat LTO, one codegen unit, panic = abort, no overflow checks')}",
        f"flags    = {toml_string('the workspace floor of .cargo/config.toml, v0 symbols, no merged functions')}",
        f"features = {toml_string(' '.join(features) or 'none')}",
        "",
        "[contenders]",
    ]
    lines += [f"{toml_string(name)} = {toml_string(version)}" for name, version in sorted(contenders().items())]
    lines += [
        "",
        "[parameters]",
        f"passes   = {passes}",
        f"filters  = {toml_string(' '.join(filters) or 'every measurement')}",
        f"inputs   = {toml_string('SplitMix64 from fixed seeds: 1,024 predictable values, 65,536 unpredictable')}",
        f"counters = {toml_string('each pass TOML names them, in its first line')}",
    ]
    return "\n".join(lines) + "\n"


def disagreements(directory, passes):
    """Each figure whose passes' medians spread wider than its agreement allows."""
    found = []
    for bench in BENCHES:
        runs = [
            {entry["name"]: entry for entry in tomllib.loads(path.read_text()).get("measurement", [])}
            for path in (directory / f"{bench}-{number}.toml" for number in range(1, passes + 1))
        ]
        for name in runs[0]:
            for figure, allowed in AGREEMENT.items():
                medians = [run.get(name, {}).get(figure, {}).get("median") for run in runs]
                if None in medians or min(medians) <= 0:
                    continue
                if (max(medians) - min(medians)) / min(medians) > allowed:
                    found.append(f"{name}: {figure} " + ", ".join(f"{median:.4g}" for median in medians))
    return found


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--pr", required=True, type=int, help="the pull request the run belongs to")
    parser.add_argument("--what", required=True, help="what the run measures, one word: ops, baseline")
    parser.add_argument("--host", required=True, help="the machine, one word: graviton4, zen5, m4")
    parser.add_argument("--passes", default=2, type=int, help="how many passes: 2, or 5 where 2 disagree")
    parser.add_argument("--purpose", default="every operation of ddust beside every contender, time and counters")
    parser.add_argument("--core", default=12, type=int, help="the isolated CPU a pass runs on, on Linux")
    parser.add_argument("--filter", action="append", default=[], help="a word a measurement's name must hold")
    arguments = parser.parse_args()

    features = ["--features", "kperf"] if platform.system() == "Darwin" else []
    binaries = build(features)
    now = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H-%MZ")
    run_id = f"{now}-pr{arguments.pr}-{arguments.what}-{arguments.host}"
    directory = HERE / "results" / run_id
    directory.mkdir(parents=True)
    prefix, placed = placement(arguments.core)
    for number in range(1, arguments.passes + 1):
        for bench in BENCHES:
            save = directory / f"{bench}-{number}.toml"
            with open(directory / f"{bench}-{number}.txt", "w") as table:
                command = [*prefix, binaries[bench], "--bench", *arguments.filter, "--save", str(save)]
                subprocess.run(command, cwd=HERE, stdout=table, check=True)
    probe = HERE / "target" / "release" / "examples" / "probe"
    (directory / "code.txt").write_text(output(sys.executable, str(HERE / "scripts" / "code.py"), "sizes", str(probe)) + "\n")
    (directory / "manifest.toml").write_text(
        manifest(run_id, arguments.purpose, placed, arguments.core, features, arguments.filter, arguments.passes)
    )
    found = disagreements(directory, arguments.passes)
    print(f"run: {directory.relative_to(HERE)}")
    for disagreement in found:
        print(f"passes disagree: {disagreement}")
    if found:
        print(f"{len(found)} figure(s) disagree: take the run again with --passes 5, unless it had five")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
