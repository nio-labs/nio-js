import re

files = ['docs/index.html', 'docs/native.html', 'docs/python.html']
for f in files:
    with open(f, 'r') as file:
        content = file.read()
    
    old_example = '<p>Example one-liner for a full React web app:</p>\n          <pre><code class="language-bash">nio-js init web --name my-app --framework React --db nio-db --ai NioAI</code></pre>\n        </section>'
    
    new_example = '<p>Example one-liner for a full React web app:</p>\n          <pre><code class="language-bash">nio-js init web --name my-app --framework React --db nio-db --ai NioAI</code></pre>\n\n          <p>Example for a backend-only server project:</p>\n          <pre><code class="language-bash">nio-js init server --name my-api --db nio-db --ai None</code></pre>\n        </section>'
    
    content = content.replace(old_example, new_example)
    
    # Also update the paragraph about auto-generating
    old_desc = 'This auto-generates a wired frontend (Vue, React, Svelte, etc.) and a NioJS backend with native tasks.'
    new_desc = 'This auto-generates a wired frontend (Vue, React, Svelte, etc.) and a NioJS backend with native tasks, or a backend-only server.'
    content = content.replace(old_desc, new_desc)
    
    with open(f, 'w') as file:
        file.write(content)

print("done")
