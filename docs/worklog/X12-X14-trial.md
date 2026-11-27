# X12／X14 试用体验修订与双账号联调

- 开工：2026-11-27（Asia/Shanghai）；负责人：lead，成员调用引导与账号路由状态分域实施。
- 前置：完整性盘点确认两处体验缺口；用户授权在现有受控条件下直接开发。X18 交接 [PR #17](https://github.com/xinchen6666/xinapi/pull/17)已复核普通合入主干。
- 范围：成员示例依据实际模型与通道使用 Responses；账号详情明确节点撤下后的停发原因，将出口观测标为历史事实。使用双成员、双账号、双节点完成各自成功调用及独立停发验证。
- 写入：New API 既有读取接口与 React 展示、对应文案、脱敏运行记录。服务端发送准入、数据库结构、正式测试及原工作目录编辑保留。
- 分支：`task/X12-X14-trial-readiness`。公共 Git 传输失败后，从已审阅且已合入的 X18 内容独立克隆；远端交付以更新主干为基底。

## 实施与结果

- 原版本实际复现了两个问题：成员复制命令使用 Chat 接口；来源批准撤下节点 A 后，数据面停发，而详情仍显示绑定与旧 IP。前后图见[成员原界面](../evidence/trial-before-member.png)、[成员修订界面](../evidence/trial-after-member.png)、[原路由详情](../evidence/trial-before-withdrawn.png)和[修订路由详情](../evidence/trial-after-withdrawn.png)。
- 成员现有模型读取接口保留 `data` 数组，按可选 `example_token_id` 查询当前登录成员自己的平台 Key，只返回示例模型与公开路径。依据现有平台组、Key 模型限制、有效状态、CPR 授组与启用渠道选择一致协议。CPR 示例固定本站 Responses／input，普通 OpenAI 示例保持 Chat／messages；同名混合协议或授权待配置时显示空态。
- 路由面板复用现有来源详情与缓存，以稳定节点 ID 和 available 判断撤下；当前来源版本推进时，已发布历史版本仍保持原语义。节点撤下显示暂停原因和明确换路提示，来源读取失败显示待确认；历史 IP 标为上次记录的出口观测。服务端准入、数据库结构与 CPR 业务代码保持原样。
- 真实 New API、CPR、Mihomo、PostgreSQL、两套 Redis 与自造 OAuth 上游实跑双成员、双账号、双节点：A／B 目标端分别观察到 `10.243.99.20`／`10.243.99.22`，并发 JSON／SSE 和各自 WSS 两轮续接成功；撤下 A 后其生成增量零，B 继续成功；恢复审批后两边继续完成。来源节点新版本和账号历史版本并存时仍走各自固定出口。
- Chromium 149 验证可信 HTTPS 登录、复制命令真实执行、授权待配置空态、节点撤下及恢复、注入来源读取 503 后的待确认状态和刷新；[390px 窄屏](../evidence/trial-mobile-withdrawn.png)保留完整提示。接口另验证他人 token ID 拒绝、模型限制、普通 OpenAI 的 Chat 示例及凭据留在服务器。
- Bun 1.4.4 类型检查 1,542 文件通过，3 个变更前端文件 lint 零问题，6 项现有概览回归和生产构建通过。Go 1.26.10 构建及 2 项既有模型列表回归通过；正式测试源码保持原样。完整事实见[结构化结果](../evidence/X12-X14-trial.json)。
- 过程中前端首次挂载只含 web 子目录，类型检查提示缺少父目录既有 fixture，改为完整模块挂载后通过；复制验证器按 shell 续行语义修正解析；一次恢复审批后的即时模型调用尚待已有账号状态更新，后续核对账号状态与完整复跑均通过。运行修正留在私有验证目录。
- 聚焦独立复核确认两条目标路径的归属与读取边界，提出普通 OpenAI 非对话模型的示例问题。已复用图像模型识别与 Responses 专属模型规则，嵌入模型也留为空态；实际平台 Key 分别限制到嵌入、图像及 Responses 专属模型后，接口返回相应空态或 Responses 示例。Go 构建及原模型列表用例再次通过。
- 交付 [PR #18](https://github.com/xinchen6666/xinapi/pull/18)，[实现版本 PR 检查](https://github.com/xinchen6666/xinapi/actions/runs/45980240614)通过。工作树原始编辑与私有配置、数据卷均保留；公开发行和正式部署另行安排。
- 参考 OWASP [授权](https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html)与[会话](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html)中的对象归属和秘密保护原则。真实第三方账号、节点协议扩展、官方客户端专项、公开发行和 X17 保留后续授权范围。
