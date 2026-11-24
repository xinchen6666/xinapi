# X06 审核：V01–V03

审核日期：2026-11-24（Asia/Shanghai）

结论：**failed，建议修订后复审**。已复现 6 项实现问题，另有 1 项部分接受确认缺口。V01 的常规格式样本通过；V02 的大小、下载时限及跳转次数限制通过，HTTPS 实际地址访问控制存在缺口；V03 的一层展开与 provider 数量限制通过，文件型资源诊断及部分接受流程需要补齐。账号、现有配置、服务进程和后台拉取的副作用检查在本轮覆盖内通过。

本轮保持产品源码原状。负责人为 lead；证据来源为当前源码阅读、隔离服务的实际 API 调用、目标端请求记录与真实浏览器操作。两个只读子任务在返回报告前结束，本文结论全部由 lead 复核和复现。

修复后续：当前实现已按 R1–R6 和确认缺口修订，并通过本地复验；详见 [修复报告](X06-fix.md)及 [E022](../VALIDATION.md#e022-x06审核修复复验)。本文保留修复前事实。

## 环境与证据范围

- 独立 Compose 项目 `xinapi-x06-audit`，使用全新数据卷、临时管理员、一个自造停用账号及一个自造代理；原评审栈保持运行
- 镜像复用 `xinapi/cpr:x06-preview`、`xinapi/new-api:x06-preview`；本轮验证运行行为，发行镜像重建留待相应交付任务
- 自造 HTTP/TLS 服务位于隔离内部网络；`source.audit.test`、`target.audit.test` 与私网 IP 均指向受控服务。专用测试 CA 只注入审核 CPR，正常证书验证保持开启
- `subscriptions.insecure_test_hosts` 只含 `source.audit.test`。私网 IP 与第二个域名均处于该 HTTP 例外名单之外
- 53 个首轮场景、8 个补充 BFF 场景及 1 个 CPR 直调对照，另有 7 次带既有业务数据的状态对照及浏览器交互
- 完整脱敏结果：[审核证据](../evidence/X06-review.json)。页面：[部分成功](../evidence/X06-review-partial.png)、[错误状态](../evidence/X06-review-error.png)
- 复现工具与生成式样本保存在私有运行目录 `.runtime/x06-audit/`，产品测试文件保持原状

## 实现问题

### R1 · P1 · HTTPS 放行只按 scheme 判断，私网实际地址可以访问

位置：[目标策略](../../services/codex-proxy-rs/backend/crates/gateway-admin/src/model/subscriptions.rs#L277)、[连接发送](../../services/codex-proxy-rs/backend/crates/gateway-host/src/subscription_fetch.rs#L52)。`https` 分支直接返回成功；下载器随后交由 reqwest 解析并连接，代码缺少地址分类检查及已审核地址的连接约束。

复现：向自造私网 HTTPS 地址直接预览、经 302 跳转访问同一私网地址、从 HTTP provider 引用该地址，均返回 200。目标端同时记录到实际连接，证据含 `private_direct`、`private_sink`、`private_provider_sink`。域名解析到私网地址的路径也实际成功。此结论针对访问策略，TLS 校验仍然开启。

影响：管理员粘贴的订阅及其跳转/provider 可以触达网关可访问的内部 HTTPS 服务，V02 的实际目标访问约束落空。

建议：对首跳、每次跳转和 provider 的解析地址应用统一访问策略；把解析、校验与实际连接绑定起来。HTTP 测试例外应与生产访问策略明确分开。

### R2 · P2 · 节点连接参数缺少校验，“可用”实际由协议名决定

位置：[Clash 节点读取](../../services/codex-proxy-rs/backend/crates/gateway-admin/src/use_case/subscriptions/parse.rs#L212)、[URI 读取](../../services/codex-proxy-rs/backend/crates/gateway-admin/src/use_case/subscriptions/parse.rs#L286)、[支持标记](../../services/codex-proxy-rs/backend/crates/gateway-admin/src/model/subscriptions.rs#L149)。

自造输入：

```text
http://#bad-http
socks5://#bad-socks
vmess://!!!!#bad-vmess
ss://garbage#bad-ss
```

实际结果：4 个候选；HTTP/SOCKS 两项 `supported=true`、`accepted=2`。VMess/SS 两项仅报告 `unsupported_protocol`，损坏的载荷得到协议限制诊断。Clash JSON 中仅含 `name/type` 的 HTTP 节点，以及 `port=70000` 的 SOCKS 节点，同样被计入可用项。

建议：分别表达格式解析结果、连接参数有效性与当前传输能力。仅在地址、端口及协议必要参数校验后把节点计入有效候选。传输能力标签应说明原生 HTTP/SOCKS 与后续 Mihomo 支持的边界。

### R3 · P2 · Base64 长度启发式拒绝合法包装文档

位置：[包装识别](../../services/codex-proxy-rs/backend/crates/gateway-admin/src/use_case/subscriptions/parse.rs#L52)。解码前要求压缩空白后的长度处于 32 字节至 2 MiB。

复现：`http://a:80#n` 的合法 Base64 包装、`proxies: []` 的合法 Base64 包装均返回 422 `subscription_unreadable`。gzip 解压后约 2.27 MB 的合法 Base64 文档也返回相同错误；它仍处于 8 MiB 解压后限额内。

建议：以解码结果及内层文档校验识别包装，保持原始下载限额、解压限额和解码限额的职责一致。

### R4 · P2 · YAML 语法失败后转入 URI 路径，整篇错误被改为部分成功

位置：[格式回退](../../services/codex-proxy-rs/backend/crates/gateway-admin/src/use_case/subscriptions/parse.rs#L83)。

自造输入：

```text
proxies: [
http://192.0.2.11:80#hidden
```

实际返回 200，`format=uri_list`、`accepted=1`、`rejected=1`。文档宣称的 YAML 结构已经损坏，HTTP 行仍被当作可用节点。

建议：把结构化文档的语法错误与格式识别失败分开；对已识别的 Clash 文档返回明确的文档级错误。

### R5 · P2 · 文件型、额外类型及错误 provider 结构被静默跳过

位置：[provider 收集](../../services/codex-proxy-rs/backend/crates/gateway-admin/src/use_case/subscriptions/parse.rs#L235)。

文件型 provider、inline provider，以及数组形式的 `proxy-providers` 均返回 200、零候选、零诊断。文件读取保持关闭，安全副作用检查通过；管理员却得到类似有效空订阅的结果。HTTP provider 缺少 URL 时会报告诊断，说明当前处理存在可观察差异。

建议：为文件型、当前支持范围之外的类型及结构错误提供明确拒绝或逐项诊断，保留来源名与稳定原因码。

### R6 · P2 · CPR 响应放大超过 BFF 限额，允许输入最终显示服务离线

位置：[候选与诊断分配](../../services/codex-proxy-rs/backend/crates/gateway-admin/src/use_case/subscriptions/parse.rs#L179)、[响应接收限额](../../services/new-api/service/cpr_readonly.go#L197)。

自造 gzip 文档含 18,000 个长名称 SS 节点，解压后 6,624,013 字节，处于 8 MiB 限额内。CPR 直调返回 200、18,000 个候选，JSON 紧凑编码响应为 12,438,217 字节。经 New API 调用返回 503 `account_egress_unavailable`，因为 BFF 只接收 8 MiB。单次实测约 5.5 秒。

建议：在 CPR 侧设置与 BFF 一致的候选、诊断或总响应预算；超额时返回明确的容量错误，避免将确定的输入规模问题映射为服务离线。

## 部分接受确认缺口

位置：[提交及结果呈现](../../services/new-api/web/src/features/subscription-preview/index.tsx#L90)、[计数展示](../../services/new-api/web/src/features/subscription-preview/components/preview-summary.tsx#L39)。

主文档一个有效节点、一个成功 provider、一个 401 provider：页面显示 `Accepted=2`、`Rejected=0`，诊断表有 `provider_fetch_failed`。真实浏览器中表单只有 `Preview` 按钮，确认弹窗数与确认选择控件数均为 0。

当前实现提供一次性查看结果，保存与发布路径留在后续任务，所以本轮没有观察到自动发布行为。用户要求的“部分接受须明确确认”仍为待满足条件；后续应让用户明确确认可接受的节点/来源范围，并区分已解析节点数量与用户批准数量。

## 格式样本表（V01）

常规样本使用 9 个节点：HTTP、SOCKS5、SS、VMess、VLESS、Trojan、Hysteria2、TUIC、WireGuard。对应名称逐项核对；URI 样本同时覆盖 fragment 百分号解码与 VMess `ps` 名称。表中的 2/7 表示当前代码的传输能力标记。

| 样本 | 实际结果 | 判定 |
| --- | --- | --- |
| Clash YAML | 9 候选；名称与协议逐项符合预期；2 可用/7 待支持；7 条诊断 | 常规解析通过 |
| Clash JSON | 与 YAML 相同 | 常规解析通过 |
| URI 列表 | 9 候选；名称与协议逐项符合预期；2/7 | 常规解析通过 |
| Base64(YAML)、Base64(JSON)、Base64(URI) | 各 9 候选；三组名称、协议与计数一致 | 常规解析通过 |
| 正常 URI + 破损行 + 额外协议 | 3 候选；1/2；逐项 `node_parse_error` 与 `unsupported_protocol` | 通过 |
| 缺少地址的 HTTP/SOCKS URI | 两项均被计入可用 | 待修订 R2 |
| Clash 缺地址、越界端口 | 两项均被计入可用；结构错误条目有诊断 | 待修订 R2 |
| 破损 VMess/SS 载荷 | 被归为协议待支持，缺少载荷解析错误 | 待修订 R2 |
| 短 Base64 URI、空节点文档包装 | 422 `subscription_unreadable` | 待修订 R3 |
| gzip 包装的大型合法 Base64 | 422 `subscription_unreadable` | 待修订 R3 |
| 损坏 YAML + URI 行 | 200，改按 `uri_list` 返回 1/1 | 待修订 R4 |
| 空正文、破损 JSON/YAML、非法 UTF-8、错误 Base64 | 422，统一脱敏错误 | 通过 |

**能力区分：**8 类协议名称得到识别，WireGuard 进入 `unknown` 并报告诊断。当前 `supported` 仅为 HTTP/SOCKS 的原生物化路径标记；SS/VMess/VLESS/Trojan/Hysteria2/TUIC 的名称识别已经存在，实际传输依赖后续引擎。本轮验证元数据解析，实际节点连通性与 Mihomo 执行属于后续任务。

## 下载边界反例表（V02、V03）

下表 HTTP 状态来自 New API BFF；CPR 原生下载失败通常为 502，BFF 将稳定订阅错误映射为 422。

| 输入/反例 | 实际结果 | 判定 |
| --- | --- | --- |
| 200 ms 慢响应 | 200 | 通过 |
| 响应头延迟 11.5 s、正文中途停顿 11.5 s | 约 10.014 s 返回 `subscription_timeout` | 通过 |
| 正文恰好 2 MiB | 200 | 通过 |
| 正文超过 2 MiB，有长度声明/流式读取两种 | 422 `subscription_too_large` | 通过 |
| gzip、deflate 正常文档 | 200 | 通过 |
| gzip 解压约 3 MiB、恰好 8 MiB | 200 | 通过 |
| gzip 解压超过 8 MiB | 422 `subscription_too_large` | 通过 |
| 损坏 gzip、额外 Content-Encoding | 脱敏 `subscription_fetch_failed` | 通过 |
| 来源 401/403，正文及头含自造私密标记 | 脱敏错误，私密标记出现次数为 0 | 通过 |
| 3 次跳转 | 200 | 通过 |
| 4 次跳转、循环跳转 | 422 `subscription_too_many_redirects` | 通过 |
| HTTP 测试名单内主机 | 200 | 通过 |
| HTTP 名单外主机，直接请求/跳转 | 422，目标命中次数 0 | 通过 |
| 跳转到 FTP | 策略拒绝 | 通过 |
| HTTPS 直接私网、跳转私网、provider 指向私网 | 全部 200，目标端记录实际请求 | 待修订 R1 |
| 域名解析到私网 | 连接成功；地址类别校验缺失 | 待修订 R1 |
| 带自造源认证的跨域跳转、302 Set-Cookie | 目标端 Authorization/Cookie/管理 Key 命中次数均为 0 | 通过 |
| HTTP provider 成功展开 | 与主文档合并为 2 候选 | 通过 |
| provider 超大正文/解压膨胀/跳转超限 | 保留主文档，返回 `provider_fetch_failed` | 通过 |
| provider 超时 | 整体约 10.014 s；后续 provider 请求次数 0 | 通过 |
| 33 个 HTTP provider | 只拉取 32 项，第 33 项有明确诊断 | 通过 |
| 嵌套与自引用 provider | 只展开一层，返回 `provider_depth_limit`；下层目标命中 0 | 通过 |
| 文件型、inline、错误结构 provider | 200，空候选、空诊断 | 待修订 R5 |
| 有效节点 + 成功 provider + 失败 provider | 保留 2 候选，失败项有诊断；页面缺少部分接受确认 | 确认项待补齐 |
| 合法限额内文档生成 12.4 MB 预览 | CPR 200；BFF 503 | 待修订 R6 |

来源 URL 原样回显会把管理员提交的 userinfo 与查询令牌带回同一管理会话。本轮未观察到它们随跨域认证头传播；回显行为与对外泄漏分别记账。

## 预览前后状态对比

状态对照在创建自造停用账号和既有代理之后采集，覆盖成功、下载失败、解析失败、超限、部分成功以及含 rules/script/controller 的文档。

| 对象 | 预览前 | 预览后 | 判定 |
| --- | --- | --- | --- |
| 账号数量 | 1 | 1 | 保持原值 |
| 账号启用与状态 | `enabled=false`、`disabled` | 同值 | 保持停用 |
| 账号出口绑定 | `null` | `null` | 保持原值 |
| 既有代理 | 1 条，revision=1，enabled=true | 同值 | 保持原值 |
| 全局配置版本 | 6 | 6 | 保持原值 |
| 服务进程 | docker-init + codex-proxy-rs | 同 PID、同名称 | 新引擎进程数 0 |
| 脚本声明的文件 | 缺失 | 缺失 | 文件写入次数 0 |
| 文档声明端口 45678/45679 | 仅输入文档声明 | 实测监听列表仍只有应用 8080 与 Docker DNS | 控制面/代理监听保持关闭 |
| 规则源、外部 UI、深层 provider | 目标记录计数 0 | 计数 0 | 额外资源拉取次数 0 |
| provider `interval: 1` | 单次展开 | 后续约 217.6 s 空闲期新增请求数 0 | 自动刷新次数 0 |
| 浏览器失败后旧预览 | 成功结果已显示 | 错误时旧行隐藏，重试按钮存在，重新提交成功 | 通过 |
| 浏览器写请求 | 提交预览 | 只出现预览端点 | 保存/发布端点调用数 0 |

源码同时支持纯预览结论：[预览服务](../../services/codex-proxy-rs/backend/crates/gateway-admin/src/use_case/subscriptions/mod.rs#L39)只持有下载端口与策略；候选、诊断在内存内组合后返回。规则、脚本、监听、路径与 controller 字段仅被作为订阅正文读取，运行配置写入路径保持关闭。

## 覆盖限制与处置

- 本轮复现 HTTPS 私网与域名解析私网路径；动态 DNS 重绑定、IPv6/链路本地地址与真实公网源列为后续验证范围
- 部分接受的批准语义、持久化和实际引擎运行尚属后续工作；当前运行期只验证一次性预览
- 原 X06 记录中的构建/测试结论属于上一轮证据；本轮采用实际服务和浏览器反例，报告重点为 V01–V03
- 临时验证曾遇到审核工具的状态码预期（CPR 就绪为 204、账号创建为 201）及短期访问令牌到期；调整工具预期、复用已创建账号、续期会话后补测，以上环境问题与产品缺陷分别处理
- 本轮结果应作为 X06 进入后续工作的修订依据。产品源码与正式测试文件保持原状，GitHub PR 的外部操作留待明确指令
- 收尾确认：本轮 `xinapi-x06-audit` 六个服务全部停止，私有复现目录与隔离数据卷保留；原 `xinapi-x04-review` 继续运行。审核记录保存在本地工作树
