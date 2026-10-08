import re

with open('README.md', 'r') as f:
    content = f.read()

old_desc = 'This will automatically generate a monorepo setup featuring your chosen UI framework (Vue, React, Svelte, Lit, etc.), a NioJS backend, and native tooling (database, AI).'
new_desc = 'This will automatically generate a monorepo setup featuring your chosen UI framework (Vue, React, Svelte, Lit, etc.), a NioJS backend, and native tooling (database, AI). You can also run `nio-js init server` to create a backend-only project.'

content = content.replace(old_desc, new_desc)

with open('README.md', 'w') as f:
    f.write(content)

print("done")
