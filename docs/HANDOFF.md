# 当前交接

## 当前：X08 审核修复已交付，保留复审入口

2026-11-26（Asia/Shanghai），R1–R9与DNS语义修复已落地，61项真实Mihomo业务及故障场景通过，见[修复记录](worklog/X08-fix.md)和[E027](VALIDATION.md#e027-x08审核修复复验)。入口归属改为account或reservation；账号使用Core原生acct_标识，预留项使用UUID。入口独立认证、制品摘要准入、TLS、持久端口、每入口监督、取消关闭和资源预算均已接通。

正常停止约49ms，启动超时期间另一入口管理操作约34ms；登记损坏与制品失配明确拒绝，双账号共用同节点仍独立认证。旧v1节点登记保留原端口占位，变更已绑定版本返回冲突。源码保持X08范围，正式测试文件保持原状。

验证栈与自造SOCKS节点已停止，原有运行栈保持。私有资料在 `.runtime/x08-audit`、`.runtime/x08-fix`。修复已推送至原X08分支，Draft PR #7正文已更新。首轮CI文档检查中的历史行号链接已固定到被审核版本，继续核对最新CI；X08保持in_progress供修复后复审。账号绑定与换路由X09继续。

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
