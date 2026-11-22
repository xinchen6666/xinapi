# 项目长期记忆

更新：2026-11-22 UTC。本页保留长期事实索引和易错点。任务进度看 [STATUS](STATUS.md)，方案理由看 [DECISIONS](DECISIONS.md)，不要在这里维护第二份状态表。

| ID | 长期记忆 | 依据／类型 | 何时复核 |
| --- | --- | --- | --- |
| MEM-01 | 朋友小团队自用，不做付费商业平台；用户允许使用Cockpit代码 | 用户第二轮；[PROJECT](PROJECT.md)，已确认范围 | 用户改变用途 |
| MEM-02 | New API统一外壳，管理员负责上游授权/账号/订阅；普通成员只用平台Key | 用户第二轮，已确认范围 | 用户改变角色/功能要求 |
| MEM-03 | 两种“订阅”不同：Clash节点来源与ChatGPT账号权益 | 领域边界 | 界面、模型命名变化 |
| MEM-04 | CPR固定基线已有账号outboundProxyId；无需重写普通代理客户端 | 固定源码事实；[UPSTREAM](UPSTREAM.md) | 更换CPR提交 |
| MEM-05 | CPR禁用代理可能回默认网络；禁用账号调度不停止OAuth自动刷新 | 固定源码/合同事实；[ARCHITECTURE](ARCHITECTURE.md) | 出口策略实现或上游升级 |
| MEM-06 | Cockpit部分节点ID基于名称；native profile的provider还能自主刷新 | 固定源码事实；[订阅专题](../research/feasibility-subscription.md) | 抽离实现或上游升级 |
| MEM-07 | 稳定本地端口不代表连接池自动换路；CPR旧连接可能继续沿旧节点 | 代码调用关系推断，未运行复现；[订阅专题](../research/feasibility-subscription.md) | egress generation或排空逻辑变更 |
| MEM-08 | 不同端口／节点名不保证不同公网IP；IP探测只证明观测时刻 | 网络边界；[PROJECT](PROJECT.md) | 供应商、双栈或探测策略变化 |
| MEM-09 | CPR管理API Key对应共同管理actor；多管理员flow owner要在BFF保护 | 固定源码事实；[后台专题](../research/feasibility-admin-integration.md) | 鉴权或OAuth实现变化 |
| MEM-10 | 服务端代理不能自动设置外部浏览器登录的网络出口 | 网络边界；[ARCHITECTURE](ARCHITECTURE.md) | 授权客户端/浏览器方案改变 |
| MEM-11 | New API为React，CPR为Vue；统一原生UI需要适配，不是复制Vue页面 | 固定代码栈；[后台专题](../research/feasibility-admin-integration.md) | 上游前端变化 |
| MEM-12 | 本目录research/checkouts是研究材料；读到实现不等于本项目已实现 | 工作区事实；[UPSTREAM](UPSTREAM.md) | 正式开发基线创建 |
| MEM-13 | 源码、文档、构建、模拟测试、真实出口与用户验收是不同证据 | [VALIDATION](VALIDATION.md)；工程规则 | 验收机制变更 |
| MEM-14 | Cockpit抽离代码不能简单改标Apache-2.0；New API/Mihomo各有许可义务 | 固定许可；[UPSTREAM](UPSTREAM.md) | 版本、复制方式、分发或用途变化 |
| MEM-15 | 部署只面向单机 Docker Compose：New API、单副本 CPR 和其 Mihomo 子进程、PG18 分库分用户、两套 Redis。无 Kubernetes/Helm 交付 | 用户选型及[部署设计](DEPLOYMENT.md) | 用户改变部署范围或组件兼容条件变化 |
| MEM-16 | 代码唯一交付远端是 [xinchen6666/xinapi](https://github.com/xinchen6666/xinapi)，两套实际产品源码将进入同一仓库，不用未发布的个人 fork 或 gitlink 代替源码 | 用户指定仓库；[开发计划](PLAN.md#x02) | 用户明确更改远端，或许可审查要求调整交付边界 |
| MEM-17 | 每个 X 步骤都需工作日志、验证结果、状态与交接、独立 Git 提交和远端 SHA 核对；本地 commit、远端分支、合入 main 和部署是四种不同事实 | 用户要求和[日志模板](worklog/TEMPLATE.md) | 交付方式或验收流程改变 |

## 不要从旧记录恢复的假设

- 不再把支付、用户购买套餐、商业售卖列成首版主线。
- 不再仅因上一轮商业假设排除Cockpit复用。
- 不把“API有Proxy字段”“IP探测成功”“健康接口正常”分别夸大为全链路固定出口、账号有效或集成成功。
- 不把普通用户无菜单视为服务端授权完成。
- 不把后台agent已退出或任务工具显示done视为本项目证据已经验收；集成者需核文件和结果。

## 文档恢复原则

默认启动只读AGENTS、STATUS、HANDOFF，再按任务引用扩展。没有聊天历史也应能继续；不自动读取AGENTS的工具使用HANDOFF中的启动提示。可持续性依赖每轮维护这些文件，不能承诺任何未知agent自动遵守。

旧事实保留适用版本；变更时新建/替代决策并标失效条件，不重写成“过去一直如此”。真实订阅URL、节点密钥与OAuth凭据不得进入记忆。
