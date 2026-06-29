import os, re
d = r"d:\rj\wasteland_project\wasteland_biology\src"
rows = []
for f in sorted(os.listdir(d)):
    if not f.endswith(".rs") or f == "lib.rs" or ".bak" in f:
        continue
    p = os.path.join(d, f)
    with open(p, "r", encoding="utf-8") as fh:
        c = fh.read()
    structs = len(re.findall(r"\bpub struct \w+", c))
    enums = len(re.findall(r"\bpub enum \w+", c))
    fns = len(re.findall(r"\bpub fn \w+", c))
    tests = len(re.findall(r"#\[test\]", c))
    size = os.path.getsize(p)
    rows.append((f[:-3], structs, enums, fns, tests, size))
print("{:<24} {:>6} {:>5} {:>5} {:>5} {:>8}".format("module","struct","enum","fn","test","bytes"))
print("-" * 60)
tot = [0,0,0,0,0]
for r in rows:
    print("{:<24} {:>6} {:>5} {:>5} {:>5} {:>8}".format(*r))
    for i in range(5):
        tot[i] += r[i+1]
print("-" * 60)
print("{:<24} {:>6} {:>5} {:>5} {:>5}".format("TOTAL", *tot))
