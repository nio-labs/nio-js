import re
import os
with open("benchmarks/run.py", "r") as f:
    lines = f.read().splitlines()

new_lines = []
skip = False
for line in lines:
    if line.startswith('BUN_CANDIDATES ='):
        skip = True
        continue
    if skip and ']' in line:
        skip = False
        continue
    if skip:
        continue
    if line.startswith('DENO_CANDIDATES ='):
        skip = True
        continue
    if line.startswith('BUN = '):
        new_lines.append('import shutil')
        new_lines.append('import os')
        new_lines.append('BUN = shutil.which("bun", path=os.environ.get("PATH") + ":" + os.path.expanduser("~/.bun/bin")) or "bun"')
        continue
    if line.startswith('DENO = '):
        new_lines.append('DENO = shutil.which("deno", path=os.environ.get("PATH") + ":" + os.path.expanduser("~/.deno/bin")) or "deno"')
        continue
    new_lines.append(line)

with open("benchmarks/run.py", "w") as f:
    f.write('\n'.join(new_lines))
