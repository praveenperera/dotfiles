"""Per-kernel GPU time of two builds from nsys SQLite exports, biggest changes first

    python3 -I nsys_kernel_diff.py CTL.sqlite[,CTL2.sqlite...] EXP.sqlite[,EXP2.sqlite...] [N]

Each side averages its exports (profile both builds with the same command and repeats,
interleaved, for example ctl-r1 exp-r1 exp-r2 ctl-r2). Prints the total kernel time, the N
kernels that got faster and the N that got slower, with call counts so a changed launch
pattern stands out. Export with `nsys profile ... --export=sqlite`
"""
import collections
import sqlite3
import sys


def kernels(path):
    con = sqlite3.connect(path)
    rows = con.execute(
        "select s.value, k.end - k.start from CUPTI_ACTIVITY_KIND_KERNEL k join StringIds s on s.id = k.shortName"
    )
    total, calls = collections.Counter(), collections.Counter()
    for name, ns in rows:
        total[name] += ns
        calls[name] += 1
    return total, calls


def side(arg):
    paths = arg.split(",")
    total, calls = collections.Counter(), collections.Counter()
    for p in paths:
        t, c = kernels(p)
        total.update(t)
        calls.update(c)
    return {n: (total[n] / len(paths), calls[n] / len(paths)) for n in total}


ctl, exp = side(sys.argv[1]), side(sys.argv[2])
n = int(sys.argv[3]) if len(sys.argv) > 3 else 10
tc, te = sum(v[0] for v in ctl.values()), sum(v[0] for v in exp.values())
print(f"total kernel time: ctl {tc / 1e6:.2f} ms, exp {te / 1e6:.2f} ms, {(te - tc) / 1e6:+.2f} ms ({te / tc - 1:+.1%})")
rows = []
for name in ctl.keys() | exp.keys():
    (c, cn), (e, en) = ctl.get(name, (0, 0)), exp.get(name, (0, 0))
    rows.append((e - c, name, c, e, cn, en))
rows.sort()
fmt = "{:+9.3f} ms  ctl {:8.3f}  exp {:8.3f}  {:>7}  calls {:g}/{:g}  {}"
for title, part in (("faster", rows[:n]), ("slower", rows[::-1][:n])):
    print(f"\n{title}:")
    for d, name, c, e, cn, en in part:
        if (d < 0) != (title == "faster") or d == 0:
            continue
        rel = f"{e / c - 1:+.0%}" if c else "new"
        print(fmt.format(d / 1e6, c / 1e6, e / 1e6, rel, cn, en, name[:80]))
