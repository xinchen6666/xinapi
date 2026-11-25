# X08 专项审核：V06、V07

审核时间：2026-11-25（Asia/Shanghai）；负责人：lead；静态复核：x08-audit-config、x08-audit-lifecycle；模型：omniai/gpt-6-astra

审查对象：`task/X08-engine-supervision` 的 `d06398d`，运行镜像 `xinapi/cpr:x08-engine`。产品源码与正式测试文件保持原状。审核按仓库开发指南、贡献与审查规范、Workspace 边界及后端生命周期要求执行，覆盖真实 Mihomo、独立数据库和自造代理节点。完整脱敏结果见[结构化审核记录](../evidence/X08-review.json)

**结论：requires_changes。X08 需要修订，V06/V07 完整通过条件仍有缺口。** Host 负责进程、Admin 负责批准节点的方向合理；已实现节点级进程与回环监听。制品准入、账号独立认证、稳定端口故障处理、连接语义和正常关闭需要优先修订

## 制品身份

| 项目 | 本次核验结果 |
| --- | --- |
| 版本／平台 | `Mihomo Meta v1.19.33 linux amd64`；ELF x86-64、静态链接；Go 1.26.10；构建时间 2026-10-21 16:44:56 UTC |
| 官方来源 | [MetaCubeX/mihomo v1.19.33](https://github.com/MetaCubeX/mihomo/releases/tag/v1.19.33)，资产 `mihomo-linux-amd64-compatible-v1.19.33.gz`，asset ID 663189170 |
| 实际执行文件大小 | 62,079,102 字节 |
| 官方压缩包大小 | 22,893,972 字节 |
| 官方 `.gz` SHA-256 | `386508bfb1c72c43b6a0eda73708ecdce82546581b0ebcb16366c53757c622b6` |
| 实际 ELF SHA-256 | `817f50b3ac6a8b1c4360c5cf2961906057f3a883d970ebbcb0028a01632b620f` |
| 来源链核对 | 官方发布接口的摘要、仓库[登记](../../third_party/mihomo.lock.json)与此次下载压缩包摘要一致；解压文件与现有执行文件一致 |
| 下载条件 | 官方下载请求超时，随后使用 gh-proxy.com HTTPS 镜像；完整压缩包按官方发布摘要核对 |
| 摘要失配场景 | 在独立 ELF 副本末尾追加审核标记，摘要变为 `c55b6ad10bfced4cafbba2b5fac67d4c631d555f9c456571d860892659d14d67`；版本文本保持原值；启动仍为 HTTP 200、`running`，运行中 `/proc/PID/exe` 摘要确认为该失配值 |

当前制品身份可以由外部审核核实；宿主启动路径的身份准入仍需实现

## 需要修订的实现问题

### R1 · P1 · 保留已批准连接的 TLS 语义

位置：[配置生成](../../services/codex-proxy-rs/backend/crates/gateway-host/src/engine/config.rs#L74)

X07 将 HTTPS 归一化为 `protocol=http`，TLS 保存在私有连接字段中；生成器却只按协议字符串 `https` 决定 TLS，遗漏已批准的 `connection.tls`、SNI 等字段

实测批准 `type:http,tls:true` 后，生成配置缺少 TLS。真实 TLS 代理场景的数据请求为 502；同端口若提供受控明文代理，数据请求为 200，代理收到有效认证字段。启动接口在两种场景都返回 200／running

修订方向：按规范化连接文档保持握手语义；生成器尚缺实现的连接选项应在启动前明确拒绝

### R2 · P1 · 保持端口登记的故障一致性

位置：[登记读取](../../services/codex-proxy-rs/backend/crates/gateway-host/src/engine/entries.rs#L36)、[登记写入](../../services/codex-proxy-rs/backend/crates/gateway-host/src/engine/entries.rs#L82)

已有登记文件读取或 JSON 解码错误会变为空集合。隔离实测：A 原端口 25100、B 原端口 25104；截断登记文件并重启后列表为空，再先启动 B 得到 25100。原稳定地址因此可以重新指向其他节点

写入路径先修改内存，再持久化。实测将登记目录设为只读，首次启动返回 503；恢复写权限后重试返回 200／running，但磁盘记录仍缺失；再次重启入口消失

修订方向：将首次初始化与既有登记故障分开处理；登记故障时保留原归属并停止新分配；持久化成功后再提交内存记录，重试应完成实际写盘

### R3 · P1 · 补齐账号入口归属及独立认证

位置：[启动请求合同](../../services/codex-proxy-rs/backend/crates/gateway-api/src/admin/engines.rs#L22)、[入口标识](../../services/codex-proxy-rs/backend/crates/gateway-admin/src/use_case/engines.rs#L40)、[监听配置](../../services/codex-proxy-rs/backend/crates/gateway-host/src/engine/config.rs#L105)

当前入口只按来源和节点建立；附带 `account_id` 的启动请求返回 422。因此此次能实测同节点和两个节点，账号／授权预留项级生命周期仍缺实现

真实数据入口接受空认证、错误认证以及 A/B 交叉认证，HTTP 均为 200；SOCKS5 握手返回 `0500`，选择免认证方式。配置中的 `secret` 只保护 controller：空、错误、交叉控制密钥为 401，正确密钥为 200

修订方向：使用账号或授权预留项作为入口 owner，为各入口配置独立认证材料。回环网络隔离和控制面鉴权各自有效，账号级认证还需独立实现；X09 可继续负责账号的 `outboundProxyId` 绑定

### R4 · P2 · 在执行前核验制品身份

位置：[启动准入](../../services/codex-proxy-rs/backend/crates/gateway-host/src/engine/mod.rs#L217)

当前只检查 `is_file()` 后执行。上述真实 ELF 失配副本仍成功运行，版本文本相同也会通过

修订方向：将受信平台、版本、发布压缩包与实际执行文件建立明确校验关系，启动前执行摘要核验，失配以固定错误拒绝

### R5 · P2 · 使用真实可用的正常关闭路径并主动回收退出进程

位置：[正常停止](../../services/codex-proxy-rs/backend/crates/gateway-host/src/engine/mod.rs#L300)、[退出扫描](../../services/codex-proxy-rs/backend/crates/gateway-host/src/engine/mod.rs#L73)

运行镜像缺少 `/bin/kill` 与 `/usr/bin/kill`。API 停止依赖外部 `kill`，执行失败被忽略，实测等待 5.075 秒后完成回收；对应源码走五秒宽限后的强制终止。对真实引擎直接发送 SIGTERM 则正常退出，退出状态为 0

SIGKILL 后、发出下一次管理查询之前，真实 Mihomo 在 `/proc` 呈 `Z` 状态；查询触发 `sweep()` 后才回收。独立观察窗口中状态保持 failed，自动重启次数为零

修订方向：保证正常信号发送能力，记录失败，并由监督任务主动 wait/reap。将引擎停止及等待纳入 Host 统一关闭阶段；本轮正常容器重启已完成 PID 和监听回收，长期遗留进程结论保留为零

### R6 · P2 · 将慢启动等待限制在所属入口

位置：[全局锁](../../services/codex-proxy-rs/backend/crates/gateway-host/src/engine/mod.rs#L179)、[停止路径](../../services/codex-proxy-rs/backend/crates/gateway-host/src/engine/mod.rs#L273)

在真实 Mihomo exec 后发送 SIGSTOP，令其初始化暂停。审核环境配置 5,000ms 启动预算，启动约 5.064 秒后返回 504 并回收该 PID；同时请求另一个已运行入口的停止和列表，两者分别耗时 10.047 秒、10.049 秒

原因是启动等待和停止等待均持有全局 Mutex，五秒启动预算与另一入口五秒强停等待串接。入队时间也在引擎启动预算之外

修订方向：全局锁仅保护共享登记更新，使用每入口串行操作及完整请求时限，停止／取消能及时作用于正在启动的实例

### R7 · P2 · 将控制端口身份纳入就绪判断

位置：[端口检查](../../services/codex-proxy-rs/backend/crates/gateway-host/src/engine/mod.rs#L220)、[就绪判断](../../services/codex-proxy-rs/backend/crates/gateway-host/src/engine/mod.rs#L154)

已有入口重启只检查数据监听端口。实测只占用 controller 端口后启动，Mihomo 日志含 `address already in use`，API 仍返回 200／running。当前就绪判断仅做数据端口 TCP 连接

修订方向：核对固定的一对端口，并用本次生成的控制凭据验证该进程身份；任一必要条件失败时回收本次实例

### R8 · P2 · 使用规范身份保持重复启动语义

位置：[事实构造](../../services/codex-proxy-rs/backend/crates/gateway-admin/src/use_case/engines.rs#L77)

Store 可将来源 UUID 的大小写形式解析为同一行，Admin 却用请求原文生成入口标识。实测仅将来源 UUID 改为大写，固定同一 node_id，再次启动返回 200，产生第二个 PID 和端口，消耗额外容量

修订方向：从已读取的来源与节点文档取规范 ID，入口 owner 规范化由 Admin 统一负责

### R9 · P2 · 明确并落实资源边界

位置：[Compose](../../compose.yaml#L74)、[引擎日志](../../services/codex-proxy-rs/backend/crates/gateway-host/src/engine/mod.rs#L226)

现有运行栈和审核栈的 Docker 实际值均为 `Memory=0`、`NanoCpus=0`，只设置 `PidsLimit=256`。实例计数可限制同时运行数量，却缺少明确 CPU／内存硬上限。引擎日志直接向数据目录 append，Docker json-file 轮转覆盖的是容器标准输出，范围与该文件独立

修订方向：给出并落实施测范围内的资源预算和引擎日志保留限制。此前“内存上限沿用容器资源边界”的记录应改为实际值；本轮只验证轻负载单／双实例

## 生命周期故障表

| 场景 | 实际结果 | 判定 |
| --- | --- | --- |
| 同一节点并发启动 | 两次 200，同一 PID 215 | 节点级幂等通过 |
| 两个节点并发启动 | 两个 running，PID 321／330，端口 25100／25104 | 节点级独立实例通过 |
| 同账号／两个账号 | API 缺少账号 owner；带 account_id 返回 422 | 账号级验证缺口 |
| 入口空／错／交叉认证 | HTTP 200；SOCKS5 选择免认证 | 失败，见 R3 |
| controller 空／错／交叉认证 | 401，正确密钥 200，两个密钥独立 | 通过 |
| 上游节点认证错误 | 实际代理拒绝凭据，数据请求 502；入口仍 running | running 仅表示本地监听，连接可用性仍需另行验证 |
| 数据端口占用 | 409 engine_port_conflict，保留固定端口 | 通过 |
| controller 端口占用 | 200 running，日志报告绑定失败 | 失败，见 R7 |
| 真实引擎 SIGSTOP 造成启动超时 | 约 5.064s，504，PID 回收 | 本入口回收通过；其他入口响应延迟见 R6 |
| 客户端断开启动请求 | 200ms 时 PID 仍存在，约 0.389s 后回收；业务端口残留零 | 此次取消场景通过 |
| API 正常停止 | 约 5.075s 后回收；外部 kill 缺失 | 正常终止路径失败，最终回收通过 |
| 直接向引擎发 SIGTERM | 退出状态 0 | 引擎自身正常终止通过 |
| SIGKILL 异常退出 | 先出现 Z，管理查询后回收；failed 保留退出信号 | 监听释放通过；主动回收缺口 |
| 双引擎 CPR 重启 | 约 3.191s；旧宿主 PID 消失，新实例均 stopped、端口保留、监听残留零 | 本轮重启通过 |
| 登记文件损坏后重启 | 空列表；B 接收原 A 端口 | 失败，见 R2 |
| 登记写失败后重试 | 503 → 200 running，磁盘缺记录，重启入口消失 | 失败，见 R2 |
| 第三个实例 | 429 engine_capacity_reached | 两实例计数上限通过 |
| 摘要失配的真实 ELF | 200 running，实际执行摘要为失配值 | 失败，见 R4 |

## 资源观测与容量边界

数值为进程 RSS 与累计 CPU 秒数，CPU 字段表示自进程启动以来的累计用时。此次未运行持续压力负载

| 场景 | PID | RSS | 累计 CPU | 冷启动 |
| --- | --- | --- | --- | --- |
| 单实例，启动后空闲约 2s | 275 | 42,926,080 B，40.94 MiB | 0.05s | 52ms |
| 双实例，启动后空闲约 2s | 275／286 | 42,926,080 + 42,639,360 B，合计 81.60 MiB | 0.05 + 0.06s | 两者 52ms |
| 双实例合计 10 次自造请求 | 275／286 | 45,096,960 + 44,994,560 B，合计 85.92 MiB | 0.07 + 0.08s | 沿用同一进程 |

10 次顺序请求耗时 1.141s，两实例累计 CPU 合计增加 0.04s。审核环境上限为 2，第三实例已验证返回 429。默认配置为 4，但本次可支持规模只陈述已测的单／双实例；内存与 CPU 硬限制仍需落实，4 实例和持续高负载均为待验证

## 已核对的部署边界

- 实际用户 UID/GID 10001:10001，`init=true`，`CapDrop=ALL`，`no-new-privileges=true`，`PidsLimit=256`
- bridge 网络；Privileged=false；Devices 空；Docker socket 挂载空；生成配置的 TUN 项空
- 数据 listener 与 controller 绑定 127.0.0.1；同网络其他容器连接返回 errno 111
- 生成规则为 MATCH,node，provider 和自动选择组均缺省；上述拒绝／TLS错误场景返回 502；测试流量由受控节点处理
- 引擎目录实际 0700，配置文件 0600。此前记录“目录与文件均600”需要更正

## 静态补充与验证边界

SOCKS `dns-mode` 在[连接配置生成](../../services/codex-proxy-rs/backend/crates/gateway-host/src/engine/config.rs#L74)中被跳过，`socks5` 与 `socks5h` 的执行配置存在折叠。此项为源码问题，SOCKS 目标地址类型的运行观测留待修订复验

本轮使用真实 Mihomo 和真实 CPR，节点代理、目标站点与身份为自造。故障注入使用实际进程 SIGSTOP、实际端口占用、独立目录的文件错误和独立 ELF 副本。真实公网出口、业务账号流量、长期吞吐、启动／停止全部时序组合均留待后续验证

两路子代理只做静态审查；lead 完成所有运行验证。审核复用了既有构建镜像，产出的是新的业务及故障证据；此前编译与 CI 结果继续作为构建记录

## 收尾与下一动作

审核服务 `xinapi-x08-audit` 已停止，数据卷与私有诊断资料保留；原有运行栈保持原状。制品路径与登记文件已恢复原值，审核结束时运行中 Mihomo 为零

本次只写本地审核报告、脱敏证据与进度交接；源码修复、Git 提交、推送及 GitHub 评论均留待用户安排。优先完成 R1–R9 并复验失败场景，再评估 X08 完成状态与 X09 启动
