# 项目进度

目标：[xinchen6666/xinapi](https://github.com/xinchen6666/xinapi)，单机 Docker Compose。更新：2026-11-22 UTC。**本表是 X 任务的唯一进度来源**；不以已有上游源码、文档或工具看板推断集成产品完成。

| ID | 状态 | 负责人／工作域 | 最近事实或验收入口 |
| --- | --- | --- | --- |
| X01 | done | lead，文档和 Git | [执行日志](worklog/X01.md)：两个原始提交已推至 `main`，Git 与 GitHub API 读回的[基线提交](https://github.com/xinchen6666/xinapi/commit/bcf6eff7632da4ce00ca48492691fae58131bcc1)一致；见 E009 |
| X02 | done | lead；x02-audit-code、x02-audit-scope 复核 | 固定源码基线已交付[PR #1](https://github.com/xinchen6666/xinapi/pull/1)；本次审查修复2项校验错误通过，16项测试通过，见[E013](VALIDATION.md#e013-x02审查修订)；未合并，X03按用户要求保持pending |
| X03 | pending | 构建与 CI | 两服务基线镜像与检查 |
| X04 | pending | Compose 与基础设施 | 五服务空环境、保留卷重启和内网隔离 |
| X05 | pending | New API Go／React | 管理员只读页与后端拒绝 |
| X06 | pending | 订阅解析／管理表单 | 自造订阅预览，首次演示 |
| X07 | pending | CPR Store／Admin | 快照、节点身份、刷新差异与迁移 |
| X08 | pending | CPR Host／Mihomo | 引擎监督、稳定入口和资源观测 |
| X09 | pending | CPR 路由／连接池 | 确认版本、换路与在途连接边界 |
| X10 | pending | CPR 出站策略 | 故障与后台请求不直连 |
| X11 | pending | OAuth／只读回执 | 授权归属、丢失响应与未知状态 |
| X12 | pending | New API 管理后台 | 完整管理员工作流与权限证据 |
| X13 | pending | 成员和授权映射 | 注册无默认上游权限、改组与撤销 |
| X14 | pending | 模型转发 | Responses、SSE／WS、工具及用量 |
| X15 | pending | 跨服务测试 | 模拟矩阵／获授权的真实验收分别记账 |
| X16 | pending | 单机交付 | 从新克隆安装、首次管理员使用 |
| X17 | pending | 备份与升级 | 隔离恢复、回退和维护窗口 |
| X18 | pending | 版本发布 | 候选标签、接手试用、实际覆盖说明 |

`pending` 尚未开工；`in_progress` 正在做；`blocked` 已开始但有具体障碍；`done` 验收证据及该步 GitHub 交付已核对。工作分支已同步不等于已合入 `main`，更不等于已发布到服务器。V01–V20 仍均未运行。已完成的 D01–D05 是前期研究，证据见[验证记录](VALIDATION.md)，不计为任何 X 业务开发成果。

定义与依赖见[开发计划](PLAN.md)，接手动作见[交接单](HANDOFF.md)。
