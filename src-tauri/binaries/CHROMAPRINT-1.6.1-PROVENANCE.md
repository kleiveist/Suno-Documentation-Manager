# Bundled Chromaprint sidecar

SunoDM bundles the upstream `fpcalc` executable from Chromaprint 1.6.1 as a
Tauri external sidecar. It is started only by the Rust backend with fixed
arguments; the application never resolves `fpcalc` through `PATH` and does not
offer a user-supplied executable path.

Upstream release: <https://github.com/acoustid/chromaprint/releases/tag/v1.6.1>

Downloaded archive hashes, verified before vendoring:

| target | upstream archive SHA-256 | vendored executable SHA-256 |
| --- | --- | --- |
| x86_64-unknown-linux-gnu | `fc16cd37a70168040bc9ceb45f1d4d1216f5a75bc4c9cf8564bea70ac6a45733` | `e7b14fbf9d544f6ba99b7aced3c07786258e09e37cfcb054a41d2a6eeb0887a7` |
| aarch64-unknown-linux-gnu | `7eaf5d655c4aa172ab28e3c870b8bb61dd2c327ac94de145676f88842cf6215a` | `9b6fb816312af0b3ca6052a973ba42f61b23e7a919dce4e3ee18e57c34bf3103` |
| x86_64-apple-darwin | `0de8947c09dd93c44cece2f5947d408136a3b6692eed726d1f109506500bd773` | `c1c368de7db49541320624d5f7d4ad827cbbaca96ee104ca6d4c4e0c917c575e` |
| aarch64-apple-darwin | `254f23cb2d290069ba1d3d28199414fbf66d2054fc2f6821c2fc62ed39470a95` | `23046544591f275c6da7b0fa57c1290535eb844df271e186e37af1715040921f` |
| x86_64-pc-windows-msvc | `735d6182b38e9f364b84ce6f4ccd682c75e2851de89735711d6b762d12b92a4e` | `00dcc56d911f2dea84737aa9dc8e2d118c9eb7a037d815d1ed001d8593e8fbee` |

Chromaprint source code is MIT licensed. The official distribution describes
the bundled decoding stack as LGPL-2.1-or-later where applicable; package and
release license notices must be retained with a distribution. See the upstream
[license notice](https://github.com/acoustid/chromaprint/blob/master/LICENSE.md).
