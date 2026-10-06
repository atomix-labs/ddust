"""What each probe's code is: its size with every function it calls, and whether it divides.

Usage: assembly.py sizes <probe> | assembly.py gate <probe>

`sizes` prints each probe of `examples/probe.rs`, named by its operation and contender: its instructions, and those of every function it
reaches by a call or a tail call but a panic's cold path, which is the code an operation brings into
the instruction cache. `gate` holds ddust's kernels to the rule that a division divides by a constant:
no division instruction and no division routine in a gated probe or anything it calls, on aarch64 or
x86_64. Both read `objdump -d` of the probe binary, built with the bench profile.
"""

import re
import subprocess
import sys

# The probes that divide only by constants, which a division instruction or routine in them breaks.
# A division by a value known only at run time, a quotient's, is not gated; nor are the 128-bit
# product and rescale, which divide by 10^k through `__udivti3` until their kernels divide by a
# reciprocal, and join this list then.
GATED = [
    f"probe::{operation}::<ddust_bench::contender::ddust::{width}>"
    for width in ("Narrow", "Wide")
    for operation in ("add", "compare", "mul_round", "rescale_round", "parse", "format", "to_f64", "from_f64")
    if (width, operation) not in {("Wide", "mul_round"), ("Wide", "rescale_round")}
]
# What a probe's name says of its contender, beyond the crate's own path.
PREFIX = "ddust_bench::contender::"

# A hardware division, on aarch64 and on x86_64.
DIVISION = re.compile(r"^(udiv|sdiv|div[bwlq]?|idiv[bwlq]?)$")
# A division routine from compiler_builtins or libgcc.
ROUTINE = re.compile(r"__(u?div|u?mod|udivmod|divmod)[sdt]i[34]$|specialized_div_rem")
# A call or a tail call: aarch64's `bl` and `b`, x86_64's `call` and `jmp`.
CALL = re.compile(r"^(bl|b|call[q]?|jmp[q]?)$")
# The cold paths a panic takes, which an operation reaches but never runs: neither counted in its
# size nor followed.
COLD = re.compile(
    r"^(core::panicking::|core::option::(unwrap|expect)_failed|core::result::unwrap_failed|"
    r"core::slice::index::|core::str::slice_error_fail|alloc::alloc::handle_alloc_error|"
    r"alloc::raw_vec::handle_error|std::panicking::|rust_begin_unwind)"
)


def functions(binary):
    """Each function's instructions, by its demangled name, from `objdump -d`."""
    out = subprocess.run(
        ["objdump", "-d", "--no-show-raw-insn", "-C", binary], capture_output=True, text=True, check=True
    ).stdout
    found, current = {}, None
    for line in out.splitlines():
        header = re.match(r"^[0-9a-f]+ <(.*)>:$", line)
        if header:
            current = header.group(1)
            found[current] = []
            continue
        instruction = re.match(r"^\s+[0-9a-f]+:\s+(\S+)\s*(.*)$", line)
        if instruction and current is not None:
            found[current].append((instruction.group(1), instruction.group(2)))
    return found


def target(arguments):
    """The function a call's arguments name, or None for a label inside one."""
    named = re.search(r"<(.*)>\s*$", arguments)
    if not named or re.search(r"\+0x[0-9a-f]+$", named.group(1)):
        return None
    return named.group(1)


def reached(found, name):
    """`name` and every function it reaches by a call or a tail call."""
    seen, pending = {name}, [name]
    while pending:
        for mnemonic, arguments in found.get(pending.pop(), []):
            callee = target(arguments) if CALL.match(mnemonic) else None
            if callee in found and callee not in seen and not COLD.match(callee):
                seen.add(callee)
                pending.append(callee)
    return seen


def probes(found):
    """The probes, by contender and then operation."""
    named = [name for name in found if name.startswith("probe::") and PREFIX in name]
    return sorted(named, key=lambda name: (name.split("<", 1)[1], name))


def sizes(found):
    print(f"{'probe':56} {'instructions':>12} {'with callees':>12} {'callees':>7}")
    for name in probes(found):
        every = reached(found, name)
        total = sum(len(found[function]) for function in every)
        print(f"{name.replace(PREFIX, ''):56} {len(found[name]):>12} {total:>12} {len(every) - 1:>7}")
    return 0


def gate(found):
    failures = []
    for name in GATED:
        if name not in found:
            failures.append(f"{name}: not in the binary")
            continue
        for function in sorted(reached(found, name)):
            if ROUTINE.search(function):
                failures.append(f"{name}: calls {function}")
            divisions = [mnemonic for mnemonic, _ in found[function] if DIVISION.match(mnemonic)]
            if divisions:
                failures.append(f"{name}: {function} divides ({', '.join(divisions)})")
    for failure in failures:
        print(failure)
    print(f"assembly gate: {len(GATED)} probes, {len(failures)} division(s)")
    return 1 if failures else 0


def main():
    if len(sys.argv) != 3 or sys.argv[1] not in ("sizes", "gate"):
        print(__doc__, file=sys.stderr)
        return 2
    found = functions(sys.argv[2])
    return sizes(found) if sys.argv[1] == "sizes" else gate(found)


if __name__ == "__main__":
    sys.exit(main())
