import re

with open('docs/index.html', 'r') as f:
    html = f.read()

# 1. Fix hero-desc
old_hero = "Offload compute to native Rust and in-process Python AI."
new_hero = "Offload compute to native Rust, dynamically bind Zig and Raw C via FFI, integrate Go cloud APIs, and run in-process Python AI."
html = html.replace(old_hero, new_hero)

# 2. Add Sidebar link
sidebar_insertion = '<a href="#hybrid-engine" class="nav-link">12. The Hybrid Engine (Zig, Go, C)</a>\n          <a href="#dependencies" class="nav-link">13. Zero-Node Dependencies & Lockfiles</a>'
html = re.sub(r'<a href="#dependencies" class="nav-link">12\. Zero-Node Dependencies & Lockfiles</a>', sidebar_insertion, html)

# Fix sidebar numbers
html = html.replace('13. Single-File Capsules (.njs)', '14. Single-File Capsules (.njs)')
html = html.replace('14. Agent Diagnostics & Stdio MCP', '15. Agent Diagnostics & Stdio MCP')
html = html.replace('15. Real-World Production Examples', '16. Real-World Production Examples')
html = html.replace('16. Ecosystem Roadmap to v1.0.0', '17. Ecosystem Roadmap to v1.0.0')
html = html.replace('17. Security, Limits & FAQ', '18. Security, Limits & FAQ')

# 3. Fix main content numbers
html = html.replace('<h2>12. Zero-Node Dependencies & Lockfiles</h2>', '<h2>13. Zero-Node Dependencies & Lockfiles</h2>')
html = html.replace('<h2>13. Single-File Capsules (<code>.njs</code>)</h2>', '<h2>14. Single-File Capsules (<code>.njs</code>)</h2>')
html = html.replace('<h2>14. Agent Diagnostics & Stdio MCP</h2>', '<h2>15. Agent Diagnostics & Stdio MCP</h2>')
html = html.replace('<h2>15. Real-World Production Examples</h2>', '<h2>16. Real-World Production Examples</h2>')
html = html.replace('<h2>16. Ecosystem Roadmap to v1.0.0</h2>', '<h2>17. Ecosystem Roadmap to v1.0.0</h2>')
html = html.replace('<h2>17. Security, Limits & FAQ</h2>', '<h2>18. Security, Limits & FAQ</h2>')

# 4. Insert section content
section_html = """
        <!-- Section: The Hybrid Engine -->
        <section id="hybrid-engine" class="doc-section">
          <h2>12. The Hybrid Engine (Zig, Go, Raw C & Python)</h2>
          <p>
            NioJS is not just a JavaScript runtime—it is a <strong>Hybrid Runtime</strong> designed to let you seamlessly blend languages in a single high-throughput event loop, without orchestrating external microservices.
          </p>
          
          <div class="hybrid-grid" style="display: flex; flex-direction: column; gap: 20px; margin-top: 20px;">
            <div class="hybrid-card">
              <h3 style="margin-top: 0;">⚡ Zig (High-Speed C-ABI FFI)</h3>
              <p>Dynamically compile and bind Zig functions directly to QuickJS for cryptographic or mathematical offloading.</p>
              <pre><code class="language-typescript">import { verify_signature } from './crypto.zig';
const isValid = verify_signature(tx.amount, tx.hash);</code></pre>
            </div>

            <div class="hybrid-card">
              <h3 style="margin-top: 0;">☁️ Go (Cloud-Native Networking)</h3>
              <p>Utilize Go's <code>cgo</code> to build shared libraries, instantly granting your TypeScript backend access to Go's unmatched Kubernetes and gRPC ecosystem.</p>
              <pre><code class="language-typescript">import { FetchClusterStatus } from './network.go';
const k8sStatus = JSON.parse(FetchClusterStatus());</code></pre>
            </div>

            <div class="hybrid-card">
              <h3 style="margin-top: 0;">💾 Raw C (Instant Legacy Parsing via TinyCC)</h3>
              <p>Embedded TinyCC (tcc) compiles raw C code in-memory at runtime. No gigabyte toolchains, just instant execution of legacy code.</p>
              <pre><code class="language-typescript">import { parse_legacy_sensor } from './legacy_parser.c';
const voltage = parse_legacy_sensor(420.5, 1.05);</code></pre>
            </div>

            <div class="hybrid-card">
              <h3 style="margin-top: 0;">🧠 Python (In-Process AI Inference)</h3>
              <p>Run machine learning models natively alongside your JS router via PyO3, sharing memory space without IPC overhead.</p>
              <pre><code class="language-typescript">import { predict_fraud } from './ml_model.py';
const risk = predict_fraud(tx.amount, tx.account_age);</code></pre>
            </div>

            <div class="hybrid-card">
              <h3 style="margin-top: 0;">🦀 Rust (Native Loop Offloading)</h3>
              <p>Use the <code>/** @native */</code> directive to have the Rust host automatically compile your heavy JS math loops into machine code.</p>
              <pre><code class="language-typescript">/** @native */
function scan_history(iterations: number): number {
  let flagCount = 0;
  for (let i = 0; i < iterations; i++) {
    flagCount = (flagCount * 31 + i) & 0x7fffffff;
  }
  return flagCount;
}</code></pre>
            </div>
          </div>
        </section>

        <!-- Section 13: Zero-Node Dependencies -->
"""

html = html.replace('<!-- Section 12: Zero-Node Dependencies -->', section_html)

# Let's also do a fallback if the comment was slightly different
if '<!-- Section 12: Zero-Node Dependencies -->' not in html and section_html not in html:
    # try replacing the h2 instead
    html = html.replace('<h2>13. Zero-Node Dependencies & Lockfiles</h2>', section_html + '\n          <h2>13. Zero-Node Dependencies & Lockfiles</h2>')
    
with open('docs/index.html', 'w') as f:
    f.write(html)
print("done")
