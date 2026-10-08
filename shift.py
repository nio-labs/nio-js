import re
import sys

def shift_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    def repl(m):
        num = int(m.group(1))
        if num >= 4:
            return f">{num+1}."
        return m.group(0)

    # Replace in nav links and h2 headers
    content = re.sub(r'>(\d+)\.', repl, content)

    with open(filepath, 'w') as f:
        f.write(content)

shift_file('docs/native.html')
shift_file('docs/python.html')
