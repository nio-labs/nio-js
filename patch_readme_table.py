import re

html_table = """<div align="center" style="overflow-x: auto;">
<table>
  <thead>
    <tr>
      <th nowrap>Workload</th>
      <th align="right" nowrap>NioJS ops/s</th>
      <th align="right" nowrap>NioJS (@native)</th>
      <th align="right" nowrap>NioJS (Rust)</th>
      <th align="right" nowrap>NioJS (Zig)</th>
      <th align="right" nowrap>Node ops/s</th>
      <th align="right" nowrap>Bun ops/s</th>
      <th align="right" nowrap>Deno ops/s</th>
      <th align="center" nowrap>Verdict</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td nowrap>HTTP GET throughput</td>
      <td align="right" nowrap>7,497,151</td><td align="right" nowrap>5,786,234</td><td align="right" nowrap>31,762,165</td><td align="right" nowrap>281,531,750</td>
      <td align="right" nowrap>12,553,037</td><td align="right" nowrap>9,394,435</td><td align="right" nowrap>14,963,340</td><td align="center" nowrap>🏆 NioJS (Zig)</td>
    </tr>
    <tr>
      <td nowrap>JSON.parse small payload</td>
      <td align="right" nowrap>469,744</td><td align="right" nowrap>386,843</td><td align="right" nowrap>3,875,988,449</td><td align="right" nowrap>3,759,389,614</td>
      <td align="right" nowrap>846,643</td><td align="right" nowrap>993,785</td><td align="right" nowrap>1,124,574</td><td align="center" nowrap>🏆 NioJS (Rust)</td>
    </tr>
    <tr>
      <td nowrap>JSON.parse large payload</td>
      <td align="right" nowrap>997</td><td align="right" nowrap>1,018</td><td align="right" nowrap>151,512,929</td><td align="right" nowrap>138,887,831</td>
      <td align="right" nowrap>3,292</td><td align="right" nowrap>4,888</td><td align="right" nowrap>4,205</td><td align="center" nowrap>🏆 NioJS (Rust)</td>
    </tr>
    <tr>
      <td nowrap>JSON.stringify small object</td>
      <td align="right" nowrap>200,917</td><td align="right" nowrap>166,330</td><td align="right" nowrap>3,703,681,890</td><td align="right" nowrap>3,184,741,078</td>
      <td align="right" nowrap>1,090,222</td><td align="right" nowrap>1,284,934</td><td align="right" nowrap>2,027,624</td><td align="center" nowrap>🏆 NioJS (Rust)</td>
    </tr>
    <tr>
      <td nowrap>JSON.stringify medium object</td>
      <td align="right" nowrap>20,065</td><td align="right" nowrap>17,210</td><td align="right" nowrap>1,515,172,049</td><td align="right" nowrap>1,265,816,877</td>
      <td align="right" nowrap>174,789</td><td align="right" nowrap>249,603</td><td align="right" nowrap>222,776</td><td align="center" nowrap>🏆 NioJS (Rust)</td>
    </tr>
    <tr>
      <td nowrap>SHA 256 hashing small buffer</td>
      <td align="right" nowrap>177,540</td><td align="right" nowrap>144,042</td><td align="right" nowrap>1,136,354,984</td><td align="right" nowrap>1,315,795,579</td>
      <td align="right" nowrap>734,751</td><td align="right" nowrap>970,007</td><td align="right" nowrap>602,176</td><td align="center" nowrap>🏆 NioJS (Zig)</td>
    </tr>
    <tr>
      <td nowrap>SHA 256 hashing large buffer</td>
      <td align="right" nowrap>2,732</td><td align="right" nowrap>2,273</td><td align="right" nowrap>855,286</td><td align="right" nowrap>693,818</td>
      <td align="right" nowrap>126,651</td><td align="right" nowrap>238,379</td><td align="right" nowrap>53,539</td><td align="center" nowrap>🏆 NioJS (Rust)</td>
    </tr>
    <tr>
      <td nowrap>Buffer copy 64 KB</td>
      <td align="right" nowrap>371,644</td><td align="right" nowrap>295,875</td><td align="right" nowrap>360,558</td><td align="right" nowrap>118,764,664</td>
      <td align="right" nowrap>37,488</td><td align="right" nowrap>80,324</td><td align="right" nowrap>31,209</td><td align="center" nowrap>🏆 NioJS (Zig)</td>
    </tr>
    <tr>
      <td nowrap>Array map plus reduce</td>
      <td align="right" nowrap>801</td><td align="right" nowrap>713</td><td align="right" nowrap>170,533</td><td align="right" nowrap>84,950</td>
      <td align="right" nowrap>7,300</td><td align="right" nowrap>26,264</td><td align="right" nowrap>6,456</td><td align="center" nowrap>🏆 NioJS (Rust)</td>
    </tr>
    <tr>
      <td nowrap>String concatenation</td>
      <td align="right" nowrap>6,925</td><td align="right" nowrap>6,122</td><td align="right" nowrap>14,797,276</td><td align="right" nowrap>377,355,286</td>
      <td align="right" nowrap>117,694</td><td align="right" nowrap>258,178</td><td align="right" nowrap>119,292</td><td align="center" nowrap>🏆 NioJS (Zig)</td>
    </tr>
    <tr>
      <td nowrap>Integer loop plus arithmetic</td>
      <td align="right" nowrap>1,076</td><td align="right" nowrap>7,197</td><td align="right" nowrap>2,404,309</td><td align="right" nowrap>2,197,850</td>
      <td align="right" nowrap>24,126</td><td align="right" nowrap>21,413</td><td align="right" nowrap>22,267</td><td align="center" nowrap>🏆 NioJS (Rust)</td>
    </tr>
    <tr>
      <td nowrap>Integer loop with randomized input</td>
      <td align="right" nowrap>1,271</td><td align="right" nowrap>1,302</td><td align="right" nowrap>2,595,448</td><td align="right" nowrap>2,279,046</td>
      <td align="right" nowrap>23,712</td><td align="right" nowrap>21,724</td><td align="right" nowrap>21,837</td><td align="center" nowrap>🏆 NioJS (Rust)</td>
    </tr>
  </tbody>
</table>
</div>"""

with open('README.md', 'r') as f:
    readme = f.read()

pattern = r'<div align="center" style="overflow-x: auto;">\n<table>\n  <thead>\n    <tr>\n      <th nowrap>Workload</th>.*?</div>'
new_readme = re.sub(pattern, html_table, readme, flags=re.DOTALL)

with open('README.md', 'w') as f:
    f.write(new_readme)

print("Updated README.md")
