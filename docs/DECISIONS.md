# 决策记录

更新：2026-11-22 UTC。`accepted` 表示有明确确认来源；`proposed` 是本轮技术建议；`superseded` 保留被替代方向。技术建议不冒充用户批准。

| ID | 状态／决定者 | 选择与理由 | 影响／复核条件 |
| --- | --- | --- | --- |
| DEC-001 | accepted／用户第二轮 | 朋友小团队非商业使用，允许采用Cockpit；Clash订阅与固定出口为核心 | 商业支付、售卖套餐退出本期；用途改变时复核许可与产品范围 |
| DEC-002 | accepted／用户第二轮 | New API为整体外壳，CPR账号授权管理放管理员后台 | 成员与管理员角色必须服务端区分；不能交付只有两个后台链接的最终体验 |
| DEC-003 | proposed／lead | New API原生React页面＋Go管理适配层；CPR保留内部Rust服务 | 避免Vue组件直接混入React、iframe双登录和浏览器持有管理密钥；X05/X11验证合同 |
| DEC-004 | proposed／lead＋subscription-plan | 抽离Cockpit纯解析/节点/稳定入口/引擎思路到CPR无界面模块；Mihomo外部进程 | 不搬桌面账户存储、WebView、系统代理和本地模型中继；逐项保留来源许可 |
| DEC-005 | proposed／subscription-plan | 首版具体节点快照，不保留运行期远端provider自动刷新；来源拉取由应用受控完成 | 完整PROFILE、自动组、文件型provider和未知外部资源不作为兜底执行；复杂格式明确报不支持 |
| DEC-006 | proposed／lead | 节点持久UUID＋不可变连接版本，名称仅展示；连接变更/节点缺失不自动改绑 | 解决名称ID和同名替换；合法凭据轮换需要明确接受新版本；精确匹配需处理重复/歧义 |
| DEC-007 | proposed／lead | must-use-proxy语义贯穿所有账号出站；禁用/删除变为停止而非默认出口 | 需要修改CPR现有投影/发送路径，单纯停调度不够；固定入口换节点还要使连接池代次失效 |
| DEC-008 | proposed／subscription-plan | 小团队首版每账号独立引擎＋稳定入口 | 减少共享重载影响；内存/CPU未知，X08测量后可用新决策替代为多listeners |
| DEC-009 | proposed／admin-integration-plan | 管理密钥仅服务端；OAuth flow本地绑定管理员/会话/账号/代理版本 | CPR AdminApiKey是共享actor，BFF负责本地owner与审计，不信任用户头 |
| DEC-010 | proposed／lead | 每成员/权限范围映射CPR Client Key，账号OAuth令牌由CPR独占 | New API保持成员配额/用量权威；内部公共Key仅用于早期连通性实验 |
| DEC-011 | accepted／lead，在文档授权范围内 | Markdown＋稳定任务ID＋STATUS唯一状态源；AGENTS/交接作为入口 | 不安装额外框架或记忆数据库；本轮落地文件；Git版本基线在X01建立 |
| DEC-012 | proposed／lead | 首版外部授权浏览器显式配置账号出口；系统区分浏览器与服务器授权路径 | 内置远程浏览器不是首版依赖；需要同IP时必须实际验证浏览器路径 |
| DEC-013 | proposed／lead，交接复核后收敛 | 新增CPR只读完成回执查询；仅已有确认回执返回completed，缺失/过期为unknown，不触发交换 | X05核对接口边界、X11冻结并实现合同；覆盖响应丢失/BFF重启；不能以账号列表推断flow已完成 |
| DEC-014 | accepted／用户确认 Docker-only 版本 | 单机 Docker Compose 五服务，Mihomo 随单副本 CPR；两库两套数据库用户、两个 Redis | 为 X04、X16、X17 的部署合同；不交付 Kubernetes/Helm |
| DEC-015 | accepted／用户指定仓库并确认计划 | 使用 [xinchen6666/xinapi](https://github.com/xinchen6666/xinapi) 存实际源码；X 步骤逐次提交、推送并校验 SHA | X01 开始；不强推，不公布凭据，不把推送说成部署 |
| DEC-016 | accepted／lead，X06 实施范围内 | 订阅预览按一次性计算提供，复用 `account_egress:read` 权限；返回字段限节点名、协议、可用性与诊断 | 与 X05 同一管理入口，权限模型保持单一来源；快照、节点 UUID 与连接版本由 X07 承担，写操作权限在 X12 评估 |
| DEC-017 | accepted／用户连续开发授权，lead 落实 X07 | 来源整体认证加密由 CPR Store 单点负责；稳定 UUID 与只追加连接版本通过规范字段直接比较；写入仅由超级管理员登录会话执行 | 2026-11-25；沿用小团队单机职责，数据库与运行数据卷共同保留；来源批准范围与后续引擎/绑定职责分别交付，见 [X07 日志](worklog/X07.md) |
| DEC-OLD-01 | superseded／被DEC-001替代 | 上一轮以商业售卖为核心，因Cockpit非商业许可建议避免复制 | 仅商业前提与优先级失效；旧源码、许可证事实不被抹掉 |

## 依据与替代方案

- DEC-003/009：[后台集成专题](../research/feasibility-admin-integration.md)。可选iframe只适合临时验证；把Go和Rust硬合成一进程没有必要。
- DEC-004–008：[订阅与出口专题](../research/feasibility-subscription.md)。仅使用CPR普通代理解决不了粘贴复杂Clash订阅的完整流程；直接照搬Cockpit完整profile又不能保证固定节点。
- DEC-010：[上一轮身份边界证据](../research/subscription-gateway-research-2026-11-22.md)及CPR Client Key作用域；没有已复现泄漏结论，设计预防与实际漏洞分别表达。
- DEC-011：[项目记忆方法](../research/project-memory-methods.md)。采用AGENTS入口、ExecPlan可恢复目标、长期harness增量验证和Spec Kit任务依赖，未声称完整部署任何框架。

## 新决策模板

记录 ID、日期（含时区）、问题、选择、候选、理由、后果、状态、决定者及来源、需要重审的条件。替代时保留旧ID并指向新ID，不静默把历史建议改写成用户要求。常规实现选择可以由执行agent在当前授权范围内决定并记录，无需把每个技术细节变成用户审批。
