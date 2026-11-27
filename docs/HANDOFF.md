# 当前交接

## 当前：X08 四类节点协议扩展

前序 [PR #18](https://github.com/xinchen6666/xinapi/pull/18)已普通合入 `main`，[主干镜像检查](https://github.com/xinchen6666/xinapi/actions/runs/45986245091)通过。用户选择 VLESS／Trojan／Shadowsocks／VMess 协议族后，本轮接通对应受管执行路径；源码与实际覆盖见[协议日志](worklog/X08-protocols.md)。

真实 Mihomo 服务端与受管客户端验证了 10 个组合，均完成 JSON／SSE／WSS 和目标端 socket 出口观测；四协议停止服务端时均停发，错误认证、错误证书主机名与额外路由选项均拒绝。格式、严格 Clippy 和 16 项架构检查通过。用户明确授权同步两份现有测试文件内 5 项协议预期后，23 项订阅/API 回归全部通过；实现与证据交付 [PR #19](https://github.com/xinchen6666/xinapi/pull/19)。

支持范围以[受管节点协议](COMPOSE.md#x08-受管节点协议)为准。Reality、Vision、gRPC、插件、smux 与真实公网账号／节点仍按专项确认，公开发行和 X17 继续保留。

## 历史：X12／X14 试用修订与双账号验证

X18 交接 [PR #17](https://github.com/xinchen6666/xinapi/pull/17)已复核普通合入 `main`，[主干检查](https://github.com/xinchen6666/xinapi/actions/runs/45964387806)通过。本轮 [PR #18](https://github.com/xinchen6666/xinapi/pull/18)在独立工作树修订成员 Responses 引导和节点撤下后的停发提示，详情见[修订日志](worklog/X12-X14-trial.md)。原首页及维护编辑保留。

双成员分别使用独立 OAuth 账号与批准节点，JSON／SSE／WSS 成功、A 停发时 B 继续、固定历史版本和用量均已实测。浏览器复制请求可执行，节点历史 IP 与待确认状态清楚区分，参考[结构化结果](evidence/X12-X14-trial.json)。

**下一步：**确认实际订阅协议与目标客户端版本后安排对应验证；现有受管执行覆盖 HTTP／HTTPS 和显式远端 DNS 的 SOCKS5。真实公网账号、其他协议、持续容量、公开发行及 X17 仍按各自授权推进。

## 历史：X18 私有候选与接手

[X16 PR #16](https://github.com/xinchen6666/xinapi/pull/16)复核补齐 Docker Engine 28.0.0 最低要求，以普通合并进入 `main`，[主干检查](https://github.com/xinchen6666/xinapi/actions/runs/45955733999)通过。源代码状态与候选制品及生产部署分别记录。

[X18 PR #17](https://github.com/xinchen6666/xinapi/pull/17)交付 [候选接手与许可说明](RELEASE.md)及[实测日志](worklog/X18.md)。`x18-rc1-linux-amd64` 的本机源码包、两套应用镜像包、许可与安装说明配套。两镜像重载后通过新空卷的 HTTPS 设置、显式管理 Key 导入、受控 OAuth、成员授权、JSON／SSE／WSS 及文档 curl 调用。

**下一步：**由所有者确认是否面向团队发行、选定分发渠道并核对对应源码及归属材料。公开版标签、生产部署和外部账号验证随授权推进；X17 继续暂缓。

## 历史：X16 单机 HTTPS 安装与首次使用

X15 已复审并普通合并至 `main`，[PR #15](https://github.com/xinchen6666/xinapi/pull/15)与[主干镜像检查](https://github.com/xinchen6666/xinapi/actions/runs/45945333027)通过。X16 使用独立新克隆，原工作目录编辑保留。

[X16 PR #16](https://github.com/xinchen6666/xinapi/pull/16)交付 [单机安装手册](INSTALL.md)、宿主 nginx HTTPS 示例和 Mihomo 只读挂载。空卷五服务实跑首次设置、显式 Key 导入、受控 OAuth、成员授组及 HTTPS JSON／SSE／WSS，覆盖证书、同源拒绝、续接归属和撤权；详见[实测日志](worklog/X16.md)与[结构化结果](evidence/X16-installation.json)。本地集成镜像及私有数据保留，生产部署单独安排。

**下一步：**X17 专项演练继续暂缓，依路线图由 X18 承接源码、镜像和文档的发行交接。公开制品分发方式、公开版本标识和正式部署由所有者确认。

## 历史：X15 系统集成与出口验证

X14 经复审以普通合并提交进入 `main`，[PR #14](https://github.com/xinchen6666/xinapi/pull/14)与[主干双镜像](https://github.com/xinchen6666/xinapi/actions/runs/45917951313)成功。原 `main` 已快进，维护文档编辑保留。

[X15 PR #15](https://github.com/xinchen6666/xinapi/pull/15)交付受控 OAuth 到成员模型调用的连续集成结果，并补齐已发布路由恢复的当次请求审计。新隔离栈实际完成来源审批、授权、分组、成员授组、调用、换路、故障停发、IPv4／IPv6 私网观测和既有 WS 撤权；23 条连续结果与 20 条管理审计关联见[X15 日志](worklog/X15.md)。

**下一步 X16：**从新克隆完成单机安装和首次管理员使用路径，复用长期私有配置、明确镜像及 Mihomo 只读挂载。公网第三方账号、节点与官方浏览器登录仍按专项授权验证，受控私网观测与公网事实分别记录。X17 专项演练继续暂缓。

## 历史：X14 模型数据面与成员用量

X13 经复审以普通合并提交进入主干，[PR #13](https://github.com/xinchen6666/xinapi/pull/13)与[主干双镜像](https://github.com/xinchen6666/xinapi/actions/runs/45883410576)成功。原 `main` 工作树已快进，用户维护编辑保持。

[X14 PR #14](https://github.com/xinchen6666/xinapi/pull/14)从更新主干交付 Responses 同步、SSE、WS 多轮与工具回填的实际链路验证，成员日志公开已确认／估算／未知用量。真实 New API、CPR、Mihomo 通过自造上游完成故障及取消验证，详见[X14 日志](worklog/X14.md)与[使用说明](COMPOSE.md#x14-responses-与成员用量)。

**下一步 X15：**从来源审批、账号授权与成员授组核对系统整体使用路径，记录受控节点及自造上游的实际出口、用量与归属。公网第三方账号和节点按专项授权验证，生产部署分别安排。

## 历史：X13 成员授权与内部身份

X12 已完成合并前审阅修订，[PR #12](https://github.com/xinchen6666/xinapi/pull/12)通过普通合并提交进入 `main`，[主干双镜像](https://github.com/xinchen6666/xinapi/actions/runs/45866915134)成功。原工作树已快进，用户维护编辑保持。

[X13 PR #13](https://github.com/xinchen6666/xinapi/pull/13)从更新主干的 `task/X13-member-access` 交付，包含显式 CPR 账号组授权与成员专属内部 Key、固定渠道准入、WS 每轮重核及续接归属检查。实际三数据库、构建与浏览器结果见[X13 日志](worklog/X13.md)，配置与管理员操作见[Compose 说明](COMPOSE.md#x13-成员授权)。成员授权空集表示撤权，平台计费组保持原语义。

**下一步 X14：**在完成 X13 PR 审阅后接通完整模型数据面，延续同一成员身份、授权范围与严格账号出口。继续验证 SSE／WS 多轮、工具回填和用量，真实公网模型与正式部署保留专项验证。

## 历史：X12 管理员工作台

[任务 PR #12](https://github.com/xinchen6666/xinapi/pull/12) 以 `main` 为目标，交付统一管理入口、批准节点独立验证、容量处理、受管换路、账号分组与额度。实际验证见[工作日志](worklog/X12.md)，当前合并和部署状态以 GitHub 与实际发布记录为准。

**X13 接续：**先在 New API 建立成员与显式 CPR 账号组的授权映射，服务端持有每人成员范围的专属 CPR Client Key。注册默认停发；撤销最后一组时禁用内部 Key，避免空组代表全账号。发起模型请求和 WS 每轮继续校验成员、有效授权及续接归属。X12 的账号池分组接口与跨服务审计可直接复用；真实公网 OAuth 与生产部署留待独立验证。

## 历史：X02–X11 主干收拢

交付入口：[收拢工作日志](worklog/X11-consolidation.md)。本轮归档协作规则与脱敏资源复盘；来源预览及管理动作、OAuth 写入分别经既有权限目录、浏览器会话与来源校验。合并方式为一个汇总 PR 的普通合并提交，原任务分支保留。

**下一步 X12：**从确认完成的 `main` 开始，先接独立批准节点验证与窄 BFF 合同，再联通账号授权、受管路由、分组、额度和引擎槽位。保持现有 OAuth 会话归属和回执；Go 审计记录管理员，CPR 记录服务主体，通过请求与操作 ID 关联。详细范围见[路线图](PLAN.md#x12)。真实第三方公网 OAuth、同出口浏览器和长期负载保留待验证。

## 历史：X11 管理员 OAuth 与回执交付

2026-11-26（Asia/Shanghai），X11接通New API会话归属操作、授权表单与CPR独立入口预留／单次交换／只读完成回执，见[X11日志](worklog/X11.md)和[E030](VALIDATION.md#e030-x11管理员oauth与回执恢复)。凭据、专用代理、路由、X09确认历史和完成回执同事务保存，响应丢失及两应用重启后只GET恢复。

38项真实运行检查、三数据库6条路径共168项、浏览器选择节点／完成／取消及浅深窄屏通过。Rust严格Clippy、构建与既有回归通过，Go构建与middleware相关回归通过，Web类型／lint／构建通过。既有Go TestSetUserPermissionsStoresOnlyOverrides期待旧权限映射，新增authorize:true导致一项断言失配；正式测试文件保持原状，此项作为明确交付差异。

[Draft PR #10](https://github.com/xinchen6666/xinapi/pull/10)已交付，分支task/X11-admin-oauth以X10为base。远端质量与镜像状态见[PR检查](https://github.com/xinchen6666/xinapi/pull/10/checks)，生产部署另行安排。真实第三方公网授权与同出口浏览器保持待验证；xAI已验证真实发送与签名拒绝路径，完整成功授权仍待验证。专用联调服务已停止，自建中继和临时MySQL容器已移除；数据卷、私有资料与可复用镜像保留。用户AGENTS.md、MEMORY.md和独立资源维护报告留在工作树。

下一动作：复审本步授权归属、回执及取消收尾，随后X12完善管理员页面闭环。已完成账号入口恢复使用同operation_id的managed-route reconcile；unknown继续只读确认，显式新授权使用新操作ID。授权期限10分钟、完成回执24小时分别判断。

## 上一轮：X10 严格出口已交付，保留复审入口

2026-11-26（Asia/Shanghai），X10追加0048托管要求迁移，Core明确Account／Reservation／Blocked，Store与Host逐次核对当前绑定和实际引擎。推理、OAuth服务端交换／刷新、额度、模型目录、资料及已建连接的新操作均接入当前准入，见[X10日志](worklog/X10.md)与[E029](VALIDATION.md#e029-x10严格出口与故障停发)。

真实双Provider传输331次观测、12项网关场景、10项投影及清理检查通过；165次故障调用的上游／默认出口增量均为零。NO_PROXY=*、引擎强制退出、旧默认客户端与存量ResponseControl撤权均已验证；默认出口校准确有记录。相关严格Clippy、构建与124项既有回归通过，正式测试文件保持原状。

分支task/X10-strict-egress已普通推送并读回，源码提交903203e；[Draft PR #9](https://github.com/xinchen6666/xinapi/pull/9)以X09为base，保留open与Draft供复审，CI以PR当前检查回执为准。私有验证材料位于 `.runtime/x10`，专用CPR、PG、Redis、双节点、默认出口记录器与目标服务已停止；原有运行栈保持。用户已有AGENTS.md及MEMORY.md工作树修改保持原状。

下一动作：核对X10源码与交付后进入X11管理员OAuth归属／回执恢复。公共元数据预热沿用部署语义，首次无本地归属凭据导入与完整OAuth预留由X11承接；真实第三方账号、公网出口、长期负载保持待验证。每次发送观测与模拟业务解析分别记账。

## 上一轮：X09 账号绑定与换路已交付，保留复审入口

2026-11-26（Asia/Shanghai），X09新增受管路由候选、批准、operation回执与对账。账号绑定仍由outboundProxyId唯一保存，Host稳定入口保持端口与凭据，发布时明确终止旧引擎在途连接，Provider按route_generation隔离新连接池。见[X09日志](worklog/X09.md)与[E028](VALIDATION.md#e028-x09账号绑定及换路)。

真实双出口验证已完成：候选失败保持原出口、HTTP2／WS旧流关闭且新连接走B、旧候选冲突、响应丢失读回、来源变动恢复、重启历史版本恢复、通用管理入口保护。格式、Clippy及既有回归通过，正式测试文件保持原状。

分支task/X09-managed-routing已通过GitHub官方接口交付 [Draft PR #8](https://github.com/xinchen6666/xinapi/pull/8)，base为X08；本地随后正常合并远端历史，保留原提交。新增0047迁移，既有文件原字节保留；新表只持有路由与操作事实。隔离CPR、PG、Redis、A/B节点和目标服务收尾停止，资料保留于 `.runtime/x09`。原有运行栈保持。

下一动作：按本步源码与交付复审，随后X10覆盖全部后台请求与故障停发。[PR CI 45690896769](https://github.com/xinchen6666/xinapi/actions/runs/45690896769)质量、CPR镜像与汇总成功，X09开发交付状态为done。当前受管路由采用显式终止在途流，候选需空闲引擎槽；公网业务与完整模型推理保持待验证。

## 上一轮：X08 审核修复已交付，保留复审入口

2026-11-26（Asia/Shanghai），R1–R9与DNS语义修复已落地，61项真实Mihomo业务及故障场景通过，见[修复记录](worklog/X08-fix.md)和[E027](VALIDATION.md#e027-x08审核修复复验)。入口归属改为account或reservation；账号使用Core原生acct_标识，预留项使用UUID。入口独立认证、制品摘要准入、TLS、持久端口、每入口监督、取消关闭和资源预算均已接通。

正常停止约49ms，启动超时期间另一入口管理操作约34ms；登记损坏与制品失配明确拒绝，双账号共用同节点仍独立认证。旧v1节点登记保留原端口占位，变更已绑定版本返回冲突。源码保持X08范围，正式测试文件保持原状。

验证栈与自造SOCKS节点已停止，原有运行栈保持。私有资料在 `.runtime/x08-audit`、`.runtime/x08-fix`。修复已推送至原X08分支，Draft PR #7正文已更新。首轮CI文档检查中的历史行号链接已固定到被审核版本，[PR CI 45667344429](https://github.com/xinchen6666/xinapi/actions/runs/45667344429)质量、CPR镜像及汇总均成功；X08开发交付状态为done，Draft PR保留修复后审阅入口。账号绑定与换路由X09继续。

## 上一轮：X08 生命周期专项审核

2026-11-25（Asia/Shanghai），按用户要求对照 V06、V07 完成真实 Mihomo 专项审核，结论 requires_changes，X08 调整为 in_progress。源码与正式测试文件保持原状；报告见 [X08-review](worklog/X08-review.md)，脱敏结果见 [E026](VALIDATION.md#e026-x08生命周期专项审核)。

已复现：TLS 字段遗漏导致明文代理认证；入口缺少账号独立认证；摘要失配 ELF 仍 200/running；登记损坏后 B 取得原 A 端口；写盘失败重试后产生仅内存入口；控制端口占用仍 running；同来源 UUID 大小写可新增进程。正常停止因缺少外部 kill 等待约5.08秒后强停；初始化暂停会将其他入口管理请求拖延到约10.05秒。容器配置 Memory/NanoCpus 均为0，资源数据只支持本轮轻负载单／双实例。

正向结果：制品发布包和执行文件身份已核实；回环网络隔离、controller 认证、实例数上限、取消后的进程回收、双引擎正常容器重启与端口保持均有真实运行结果。完整失败表和资源数据以审核报告为准。

审核栈 xinapi-x08-audit 已停止；故障注入用的制品路径与登记文件已恢复，数据卷和私有资料保留在 `.runtime/x08-audit`。原有服务保持原状。下一动作是按 R1–R9 修订并复验失败场景，再评估 X08 完成状态与 X09；本次 Git 与 GitHub 写入为零。

## 上一轮：X08 首个增量交付记录

管理员可对已批准节点启动受管 Mihomo 入口：入口只监听 CPR 回环地址，端口成对分配并写入运行数据卷 `engines/entries.json`，重启后保持同一端口；配置按当前批准版本重写为最小文件（唯一节点、`MATCH,node`、`0600`）。容量、启动超时、端口占用停发、异常退出回收与资源观测均已接通，管理 API 为 `GET /api/admin/engines`、`POST /api/admin/engines/start`、`POST /api/admin/engines/stop`。

实测：真实 Mihomo v1.19.33 冷启动 52ms、RSS 约 42MB；经入口的请求由自造节点代理转发（代理计数 2→3）；回环隔离 errno 111；并发启动得到唯一 PID；`kill -9` 后状态 failed 并记录信号、端口释放、重启复用原端口；端口占用返回 409 `engine_port_conflict`；第三个实例返回 429；CPR 重启后入口全部 stopped、端口保持。证据见 [E025](VALIDATION.md#e025-x08受管引擎入口)与 [X08 日志](worklog/X08.md)。

分支 task/X08-engine-supervision 已推送，[Draft PR #7](https://github.com/xinchen6666/xinapi/pull/7) 以 task/X07-subscription-snapshots 为 base，[CI 45599508042](https://github.com/xinchen6666/xinapi/actions/runs/45599508042) 全部成功，X08 置 done。下一动作：X09 把账号出口绑定到稳定入口，并验证换路时的连接代次与在途流边界；严格出口与故障停发由 X10 承担。

运行状态：xinapi-x07-dev 栈当前使用 xinapi/cpr:x08-engine 镜像并挂载本机 Mihomo 制品，入口进程已停止；自造节点代理容器已移除，验证脚本位于 .runtime/x08。x07-fresh 与 x07-upgrade 栈保留。

## 上一轮 X07 专项复审修订

X07 来源与已批准版本的首个增量经两路独立只读复审，三项 P2 事实已修订：socks5h 的 DNS 解析位置进入连接身份（dns-mode），username/password/tls 的畸形类型在解析阶段拒绝，来源编辑器按打开时 revision 提交并在服务端版本推进时得到 409。复验通过六点专项 17 项、权限与存储 6 项、后端语义 2 项、cpr 重启 61 个来源一致，以及真实页面编辑冲突与主流程，页面运行错误零。证据见 [E024](VALIDATION.md#e024-x07专项复审修订)。

审计环境把 CRITICAL_RATE_LIMIT 调至 200 次/20 分钟用于重复运行；产品默认 20 次/20 分钟。修订以普通提交送达 task/X07-subscription-snapshots（git push 的 HTTPS 连接中断后改用 GitHub 官方内容接口，远端内容与本地逐一相等）并更新 [Draft PR #6](https://github.com/xinchen6666/xinapi/pull/6)（base 为 task/X06-subscription-preview）；[最新 PR CI](https://github.com/xinchen6666/xinapi/actions/runs/45578301022) 的质量检查、两个镜像与汇总全部成功。

X07 状态为 done。下一动作：按 [PLAN](PLAN.md#x08) 启动 X08，先确认 CPR Host 写入范围、Mihomo 版本与摘要登记方式，再做单引擎启动、认证、正常关闭与异常回收验证；账号绑定由 X09 推进，出口策略由 X10 推进。

临时服务状态：xinapi-x07-dev、xinapi-x07-fresh 与 xinapi-x07-upgrade 运行栈保留；私有配置、身份与验证脚本位于 .runtime/x07 与 .runtime/x07-review。协作入口和长期记忆的用户编辑继续独立保留。

## 上一轮 X07 首个增量交付记录

已实现来源创建/编辑/读取、受控刷新差异、显式选择身份和批准版本。Admin 以规范连接字段直接匹配，Store 单点 AES-256-GCM 密文保存来源与连接；稳定节点 UUID 与只追加连接版本分离，改名/排序/等价默认值保留身份。完整缺失需确认后标记 available=false，部分结果保留旧节点；旧刷新/旧批准受 revision 和 review_id 校验。

New API 复用「订阅预览」页提供「来源管理」：管理员只读；超级管理员登录会话执行写入，个人访问令牌仅保留读取职责。URL只作为提交字段，列表与详情保持元数据投影。密钥在 CPR 运行数据卷内 `subscription.key`（0600、32字节），已有数据时缺少或错误密钥拒绝读写。

实际结果：15项业务流程、6项补充、真实桌面/移动、密钥、重启与迁移场景通过；Rust订阅25项/API7项/架构16项、Go权限路由、前端构建和类型检查通过。整仓前端56项失败/超时和controller整包超时分别保留。范围及截图见 [X07日志](worklog/X07.md)与 [E023](VALIDATION.md#e023-x07来源与批准版本)。

当前分支 task/X07-subscription-snapshots，首个增量以 [Draft PR #6](https://github.com/xinchen6666/xinapi/pull/6)交付，base 为 task/X06-subscription-preview。完整55个源码、文档和截图文件经官方 GitHub 接口送达，工作日志及截图已回读；Git传输低速后使用本机受信凭据完成官方接口提交。X07保持in_progress，下一动作是专项复审来源身份、批准语义和密钥生命周期；账号绑定、实际出口及引擎接入依既定后续任务推进。

原 X04 运行栈保留。验证使用独立 xinapi-x07-dev 与 xinapi-x07-upgrade；私有配置、身份、源码验证工具及日志位于 .runtime/x07 与 .runtime/x07-upgrade，临时开发、迁移和密钥验证服务已停止，隔离数据卷继续保留。协作入口和长期记忆的用户编辑继续独立保留。

## 上一轮 X06 交付记录

原交接中的 X06 完成判断先由 E021 复核，当前功能修复结果见 E022；上一轮构建、接口演示和交付详情见 [X06 日志](worklog/X06.md)。

以下为 X03 历史交接材料。

## X03 历史交接
X03的CPR版本修订已完成。本轮交付统一入口的版本传递、CI版本输出比较及实际运行记录。X04保持pending，后续按用户确认的任务安排推进。

## 当前版本修订

统一入口从所选提交读取发行值3.54.2并传入镜像；本机重建与CI同一版本命令实际输出3.54.2，exit0。旧空版本镜像在新比较步骤返回exit1。现有47项工具回归通过，测试文件保持原样。

[PR CI 45120537811](https://github.com/xinchen6666/xinapi/actions/runs/45120537811)及[push CI 45120533870](https://github.com/xinchen6666/xinapi/actions/runs/45120533870)全部作业成功；两轮CPR日志均记录 `CPR release version verified: 3.54.2`。详细过程见[X03工作日志](worklog/X03.md#cpr-版本审查修订)和[E015](VALIDATION.md#e015-x03cpr-发行版本修订)。协作入口和记忆的既有本地编辑独立保留。


## 已验证成果

- 基线是X02已审查提交 `072e434a528bb9295d9c6d4ee6000c91b03485ca`；原services源码、逐文件manifest、依赖锁及许可文本未改。构建的根适配只控制同digest网络通道、资源、来源标签和许可证封装。
- New API、CPR镜像本地实际构建及无网只读短命CLI通过；CPR版本3.54.2，New API原VERSION空blob未伪造。镜像ID、artifact、平台和许可对照见[New API证据](evidence/X03-new-api.json)、[CPR证据](evidence/X03-cpr.json)。
- 提交 `a1da347925957c6f1a0ec50bd8157bb55457246a` 的[GitHub run 45104467732](https://github.com/xinchen6666/xinapi/actions/runs/45104467732)质量、两个镜像及汇总均成功，包含47项工具回归；严格绑定BuildKit导出、加载镜像、平台和源码标签。最终记录只改文档/证据，不重复宣称编译。
- 用法见[构建说明](BUILD.md)，实际过程/失败/取消/成功见[X03日志](worklog/X03.md)和[E014](VALIDATION.md#e014-x03实际镜像构建与持续检查)。源码树通过、可执行CLI和服务集成是不同证据，V01–V20未运行。

## 分支和接手

X03审阅入口为[PR #2](https://github.com/xinchen6666/xinapi/pull/2)，工作分支为 [task/X03-build-ci](https://github.com/xinchen6666/xinapi/tree/task/X03-build-ci)，依赖[PR #1](https://github.com/xinchen6666/xinapi/pull/1)。PR #2以X02分支为base，审阅范围聚焦本步构建与CI。合并顺序为先处理X02，再将X03调整至main，并依据实际PR状态开展后续工作。

进入X04前再查PR/main真实状态，从已验证的X03基线开始。X04将建立New API、CPR、PG18分库分用户、两个Redis的根Compose，默认只对本机发布New API。当前没有根Compose、真实账号、订阅或固定出口功能验收。

## 运行边界

本步没有部署业务、数据库或Mihomo，只有隔离短命命令；全部构建后台job已回收。本机既有容器未删除/重置。`.runtime/`保存临时工具与日志，不提交，真实GitHubtoken不进入Docker配置、build args、源码或CI。

Docker29的Id可能是OCI index而非config digest；不能机械把二者比较为同一值。历史来源验证用包含当前schema的X02提交，不用第一次缺import_tree字段的旧导入提交。APT安全更新/时间等可能影响字节，实际镜像用ID/digest识别，不保证跨日期逐字同构。
