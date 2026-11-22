# X02：Cockpit 与 Mihomo 采用清单

**本步交付来源、接口选择、许可正文和制品元数据，不导入 Cockpit 业务代码，不引入 Mihomo 二进制。** 结论适用于朋友小团队非商业使用，但源码将进入公开 GitHub 仓库，公开源码与分发 Docker 镜像应分别核查。此清单是工程采用门槛，不代替权利人的书面许可或法律意见。

## 版本与许可证据

| 对象 | 核实的来源 | 本仓库保留内容 |
| --- | --- | --- |
| Cockpit Tools 1.4.2 | [提交 c004bf71be250cc2536fc91ef90638837f0013a0](https://github.com/jlcodes99/cockpit-tools/tree/c004bf71be250cc2536fc91ef90638837f0013a0)；[作者 jlcodes 和 Cargo 许可字段](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/Cargo.toml#L1-L7)；[README 的许可证段](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/README.md#L515-L521) | [上游声明原文](LICENSES/cockpit-license-notice.zh.md)为固定提交的完整许可段；[CC BY-NC-SA 4.0 完整法律文本](LICENSES/cockpit-CC-BY-NC-SA-4.0.txt)取自 [Creative Commons 官方纯文本](https://creativecommons.org/licenses/by-nc-sa/4.0/legalcode.txt)。固定提交的根目录不存在项目级 `LICENSE`；仓库里的其他第三方许可证不替代该声明。 |
| Mihomo v1.19.33 | [Git 标签引用](https://api.github.com/repos/MetaCubeX/mihomo/git/ref/tags/v1.19.33)直接指向 commit `3f27ef39f8c96bbfbc8b0b9bb10f5c9ef60149d8`；[固定源码归档](https://github.com/MetaCubeX/mihomo/archive/3f27ef39f8c96bbfbc8b0b9bb10f5c9ef60149d8.tar.gz)；[该提交的 LICENSE](https://github.com/MetaCubeX/mihomo/blob/3f27ef39f8c96bbfbc8b0b9bb10f5c9ef60149d8/LICENSE) | [GPLv3 完整原文](LICENSES/mihomo-GPL-3.0.txt)，与上游 Git blob `f288702d2fa16d3cdf0035b15a9fcbc552cd88e7` 字节一致。[机器可读锁](mihomo.lock.json)只锁定外部制品，不记录任何实际运行结果。 |

CC 法律文本为 20,850 字节，SHA‑256 `e66c269d4819aaab34b49ef5220c4ddab6756f21bb5180761a4eb8561f2b7bbd`；原样保留的 Cockpit 声明为 557 字节，SHA‑256 `5c814de7e0fc4dbeacaa4e4e8e16c3ba745471c6df5219a7dbd12162e200739f`。Mihomo 许可为 35,149 字节，SHA‑256 `3972dc9744f6499f0f9b2dbf76696f2ae7ad8af9b23dde66d6af86c9dfb36986`。这些是**许可文件**摘要，不是程序摘要。

## 为什么现在不复制模块

1. 小团队非商业用途并不排除按 Cockpit 条款使用或修改；用户同意采用该设计也不等于获得权利人重新许可其实现的权利。CC BY-NC-SA 4.0 §2(a)、§3(a/b)限制非商业使用，分享时要保留署名、来源、变更说明，分享改编材料还要遵循相同许可要素或经指定的兼容许可。
2. [CC 指定兼容许可列表](https://creativecommons.org/compatible-licenses/#by-nc-sa)未列出 BY-NC-SA 4.0 的非 CC 兼容许可。BY-SA 4.0 对 GPLv3 的单向兼容说明**不是** BY-NC-SA 4.0 的授权。不能将复制或翻译的 Cockpit 代码只标为 Apache-2.0、GPLv3 或 AGPLv3；也不能给整个仓库加一个统一的 Apache 或 NC 标签。
3. 公开仓库中分别存放独立程序，与编译／链接成单一派生作品，并不是同一问题。CPR 采用 [Apache-2.0](https://github.com/zyycn/codex-proxy-rs/blob/58cbb607505dda4a1cee6e01b9f76e267ad3c261/LICENSE)；New API 采用 [AGPLv3](https://github.com/QuantumNous/new-api/blob/8542c9d6490b4330fe18160930f81ae7428bc777/LICENSE)，另有 [NOTICE](https://github.com/QuantumNous/new-api/blob/8542c9d6490b4330fe18160930f81ae7428bc777/NOTICE)。这里未确认将 CC 改编代码直接复制、静态链接或合并进上述程序后，能同时满足相同方式共享、非商业限制及各自发布条件。单独 crate、子进程、容器或目录本身均非自动豁免。
4. **当期可做：**保留许可与出处，记录需要实现的输入输出和行为约束；后续可依据公开规范及本项目合同独立实现，或由权利人明确授予适用于该组合及分发方式的许可后再引入对应文件。独立实现不逐行翻译或照搬原测试实现。若采用明确独立的 CC 组件，须另行评估边界、第三方例外、合法使用范围、文件级归属和分发方式；确认前不把实现放进服务树。本次延迟的是**复制具体实现**，不是撤销对 Cockpit 非商业设计路线的选择。
5. Mihomo 计划经标准代理接口作为独立程序使用。仅锁元数据不等于批准把程序放进镜像。实际分发二进制或镜像前，按 GPLv3 §§4–6 留存归属、许可、与确切制品对应的完整可机器读取源码（含适用的构建／安装材料、依赖声明和本方补丁）及可持续获取方式。上面源码链接是官方起点，不能仅凭一个 tag 链接就证明已满足对应源码交付。修改后运行的 New API 向远程用户提供对应源码的义务另按 AGPLv3 §13 执行。

## 逐文件采用决策

固定目录为 [Cockpit `src-tauri/src/modules/`](https://github.com/jlcodes99/cockpit-tools/tree/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/src/modules)。下列为与该功能有关的文件，**所有行在 X02 都是“未复制”**。“复用候选”指技术上可移植、许可确认后才可取实现；“改造”指保留需求但重设计服务端合同；“舍弃”指不引入。接口名是 xinapi 设计名，不声称已有对应 crate 或服务。

| 上游文件／模块 | 决策与后续任务 | 需要的接口与具体边界 |
| --- | --- | --- |
| [codex_proxy_catalog.rs](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/src/modules/codex_proxy_catalog.rs) | 改造：X06/X07 | 拆分为受控下载和候选快照；保留有界解码、超时、逐项错误、成功才发布的行为。去除桌面 `account`、`atomic_write`、`secure_account_storage`、文件锁及进程静态定时刷新；来源及版本存 CPR Store。服务端逐跳校验目标，禁止携带账号秘密下载订阅。 |
| [codex_proxy_subscription_parser.rs](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/src/modules/codex_proxy_subscription_parser.rs)；`codex_proxy_subscription_diagnostics.rs` | 复用候选／改造：X06/X07 | `parse(bytes) → 候选节点、可解释错误`；候选覆盖 Clash YAML/JSON、Base64 文档、URI 列表。预览不启动引擎；明确区分部分解析、无效输入与发布。上游用名称派生的 ID 不代替持久 UUID＋不可变节点连接版本。 |
| [codex_proxy_node_parser.rs](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/src/modules/codex_proxy_node_parser.rs) | 复用候选：X06 | 单个节点 URI 到规范化节点；边界主要是 `url`、Base64、Serde JSON。禁止在错误中回显链接；协议或选项只按实际 fixture／传输验证开放。 |
| [codex_proxy_mihomo.rs](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/src/modules/codex_proxy_mihomo.rs)、[codex_proxy_network.rs](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/src/modules/codex_proxy_network.rs) | 复用候选／改造：X06/X08 | 可借鉴节点字段校验与旧记录的转换职责；不采用旧桌面快照作为公共存储合同。由服务端允许列表生成单一具体节点配置，TLS 验证保持开启；不接受订阅指定控制面或监听设置。 |
| [codex_proxy_native_profile.rs](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/src/modules/codex_proxy_native_profile.rs)、[codex_proxy_engine_native.rs](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/src/modules/codex_proxy_engine_native.rs)、`codex_proxy_native_assets.rs`、`codex_proxy_verge_compat.rs` | 舍弃完整 profile 执行路径：X06 | 区分纯解析与启动临时 Mihomo 检查配置；远端 provider 由受控下载器展开成静态节点。首版不加载原 rules、原生 provider 自刷新、任意本地文件、额外 controller 或内核链。 |
| [codex_proxy_catalog_binding.rs](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/src/modules/codex_proxy_catalog_binding.rs)、`codex_proxy_manual_import.rs`、`codex_proxy_strategy.rs` | 改造绑定；舍弃桌面导入／多出口策略：X07/X09 | 账号→代理只由 CPR `outboundProxyId` 持有；代理→具体节点版本由受管路由拥有。接口接受未变更的预览代次、显式确认新版本；不复制私有 Binding URL 格式，不用策略组自动临时改绑。 |
| [codex_proxy_engine.rs](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/src/modules/codex_proxy_engine.rs)、`codex_proxy_engine_errors.rs`、`codex_proxy_engine_preflight.rs`、`codex_proxy_engine_process.rs` | 复用生命周期设计，改造：X08 | `prepare → ready → publish / stop`；Host Tokio 管理子进程，不链接 Go 内核；受限私有目录、配置经私有通道、启动失败无泄密。有界并发、取消、回收由容器内 Linux 验收；不移植 Windows 桌面进程控制。 |
| [codex_proxy_engine_install.rs](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/src/modules/codex_proxy_engine_install.rs)、`codex_proxy_engine_archive.rs`、`sidecars/mihomo/upstream-assets.json` | 改造：X08，X16 前分发审查 | 采用“先验证压缩摘要与大小，再有界解压”的合同；输入改用本项目专有锁。去除 Tauri 用户触发的在线安装和可变 active 指针。宿主检查与产品镜像发布是不同步骤，当前不提供也不执行下载器。 |
| [codex_proxy_runtime.rs](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/src/modules/codex_proxy_runtime.rs)、`codex_proxy_runtime_request_routes.rs`、`codex_proxy_runtime_latency.rs` | 改造：X08–X10 | 每账号启动合并、不可变租约、就绪后发布、旧连接排空；替换 `CodexAccount`、Tauri 异步任务、全局 map、活动日志及磁盘访问。CPR 连接池感知出口代次；已有连接上的新请求也必须受停发约束。 |
| [codex_proxy_desktop_router.rs](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/src/modules/codex_proxy_desktop_router.rs)、`codex_proxy_desktop_protocol.rs`、`codex_proxy_desktop_routes.rs` | 复用稳定入口设计，改造：X08–X10 | 保留 TCP SOCKS5／HTTP CONNECT 入口对应特定账号和出口租约的语义；端口分配、认证和绑定状态改由受管服务控制。删除“无绑定直连”；冲突时停发或原子更新。节点缺失、代理禁用、引擎退出均不转全局出口。 |
| `codex_proxy_probe.rs`、`codex_proxy_engine_delay.rs`、`codex_proxy_activity.rs` | 改造：X09/X12 | 只保留受控、脱敏、与版本关联的探测结果和近期错误；复用 CPR 已有出站观测，不迁移桌面监视器。可达、当前公网 IP 和永久独占 IP 是不同结论。 |
| `codex_proxy_direct_hosts.rs`、`codex_proxy_dotenv.rs`、`codex_proxy_desktop_entry.rs`、`codex_proxy_desktop_processes.rs`、`codex_proxy_desktop_recovery.rs`、`process_codex_proxy_snapshot.rs` | 舍弃 | 不改系统/桌面进程环境、`.env`、直连域名和客户端启动参数，亦不复制桌面故障恢复逻辑。 |
| [codex_account_proxy.rs](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/src/modules/codex_account_proxy.rs)、`codex_proxy_targets.rs`、`codex_oauth.rs`、`codex_quota.rs`、`codex_local_access.rs` | 舍弃第二套账号／API 网关 | 仅审阅账号出站覆盖面；上游 OAuth、令牌、组、限额和推理由 CPR 原生服务拥有。New API 持有成员角色和可信内部 Key 映射；不搬旧 OAuth 浏览器、下游中继或数据库。 |
| [Tauri 管理 commands](https://github.com/jlcodes99/cockpit-tools/tree/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/src/commands)（`codex_proxy_catalog.rs`、`codex_proxy_engine.rs`）；[桌面 React 组件](https://github.com/jlcodes99/cockpit-tools/tree/c004bf71be250cc2536fc91ef90638837f0013a0/src/components/codex)及 `src/services/codexProxy*Service.ts` | 舍弃 IPC、UI 实现 | 仅以功能作为需求输入；New API 原生 React 页面通过有鉴权的 Go BFF 调用 CPR。节点导入、绑定和重启不能仅靠菜单隐藏或浏览器持有管理密钥。 |

[上游 Cargo 依赖](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/Cargo.toml#L13-L101)还包括 `tauri-build`、Swift 构建、Wry、剪贴板/托盘/深链等；不是都要带入 Linux 镜像。未来的纯解析单元只需按实际 API 选 Serde、YAML/JSON、`url`、Base64；下载与流由受限 HTTP 客户端和 Tokio 管理；摘要、受限解压、随机认证和 TLS 相关依赖按各自用途引入、复核许可并锁定版本。在核心热路径不引入 Tauri。以上没有宣称新库已提取、构建或通过测试；后续落实在 [X06、X08–X10](../docs/PLAN.md#x06)。

## Mihomo 锁的使用边界

- 上游 [v1.19.33 release](https://github.com/MetaCubeX/mihomo/releases/tag/v1.19.33) 的 [GitHub Releases API](https://api.github.com/repos/MetaCubeX/mihomo/releases/tags/v1.19.33) 返回 release ID `419818179`、`immutable=false`。锁按 `tag → commit` 和 `平台 → asset ID／名称／size／digest` 固定，不把 `target_commitish=main` 当版本。以后获取发现字段或摘要漂移应停止并重新审查，不自动接受新资产。
- `linux/amd64` 选官方 `amd64-compatible`；`linux/arm64` 选无额外变体的包。[Cockpit 自己的资产清单](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/sidecars/mihomo/upstream-assets.json#L18-L27)在 amd64 选的是 `amd64-v1`，压缩字节和 digest 与本项目选择不同；不要交换校验值。`compatible` 是上游制品标签，不是本项目实测的 CPU/系统兼容承诺。
- `sha256` 和 `size_bytes` 对应下载的 **`.gz` 原始压缩字节**，来自官方发布 API，不是解压后可执行文件的哈希、签名或本地重算值。两平台的 `unpacked_sha256=null`。本步 `binary_not_downloaded=true`、`not_executed=true`、`build=false`；没有拉取可执行包、解压、执行版本探测、验证二进制签名、构建镜像或批准捆绑。
- X08 实际获取时先核对锁定的名称、长度与压缩 SHA‑256，再有界解压、记录可执行文件摘要并在获授权的对应平台测试；X16/X18 在分发前完成上文 GPL 对应源码及依赖材料检查。[Cockpit 自己的审查记录](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/docs/third-party/mihomo-v1.19.33.md#L1-L19)同样区分主动下载与发行包内置，不能当作 xinapi 的发行许可。
