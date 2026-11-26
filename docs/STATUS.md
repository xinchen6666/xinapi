# 项目进度

目标：[xinchen6666/xinapi](https://github.com/xinchen6666/xinapi)，单机 Docker Compose。更新：2026-11-27（Asia/Shanghai）。**本表是 X 任务的唯一进度来源**；主干收拢与部署状态分别见[交接单](HANDOFF.md)。

| ID | 状态 | 负责人／工作域 | 最近事实或验收入口 |
| --- | --- | --- | --- |
| X01 | done | lead，文档和 Git | [执行日志](worklog/X01.md)：两个原始提交已推至 `main`，Git 与 GitHub API 读回的[基线提交](https://github.com/xinchen6666/xinapi/commit/bcf6eff7632da4ce00ca48492691fae58131bcc1)一致；见 E009 |
| X02 | done | lead；x02-audit-code、x02-audit-scope 复核 | 固定源码基线已交付[PR #1](https://github.com/xinchen6666/xinapi/pull/1)；本次审查修复2项校验错误通过，16项测试通过，见[E013](VALIDATION.md#e013-x02审查修订)；未合并，X03现按用户继续指令推进 |
| X03 | done | lead；CPR版本修订复核完成 | [PR #2](https://github.com/xinchen6666/xinapi/pull/2)包含R1修订；本机及[新CI](https://github.com/xinchen6666/xinapi/actions/runs/45120537811)实际版本输出3.54.2，全部作业成功；47项原工具回归通过，见[E015](VALIDATION.md#e015-x03cpr-发行版本修订)。X04待启动 |
| X04 | done | lead；修订已交付 | [PR #3](https://github.com/xinchen6666/xinapi/pull/3)已同步真实就绪启动与原卷补全修订，远端代码及文案读回通过，复验见 E018；保持 open，base 为 X03 |
| X05 | done | lead；Draft 已交付 | [PR #4](https://github.com/xinchen6666/xinapi/pull/4)已推送并读回，base 为 X04；构建、真实 CPR、七语浏览器验证及 authz 全量回归通过，见 [E019](VALIDATION.md#e019-x05管理员只读入口) |
| X06 | done | lead；审核修复已交付 | R1–R6 与部分接受确认完成，72 项接口及浏览器复验通过；源码与截图已普通推送至 [Draft PR #5](https://github.com/xinchen6666/xinapi/pull/5)，见 [E022](VALIDATION.md#e022-x06审核修复复验)和[修复报告](worklog/X06-fix.md) |
| X07 | done | lead；专项复审修订已交付 | 来源密文、稳定节点ID、刷新差异及批准完成；复审三项修订通过 17+6+2 项、重启与真实页面复验，见 [E024](VALIDATION.md#e024-x07专项复审修订)；[Draft PR #6](https://github.com/xinchen6666/xinapi/pull/6) 已含修订 |
| X08 | done | lead；审核修复已交付 | R1–R9及DNS语义修复完成；61项真实引擎场景、既有回归与[PR CI](https://github.com/xinchen6666/xinapi/actions/runs/45667344429)通过，见 [修复记录](worklog/X08-fix.md)与[E027](VALIDATION.md#e027-x08审核修复复验)；[Draft PR #7](https://github.com/xinchen6666/xinapi/pull/7) 保留修复后审阅入口 |
| X09 | done | lead；账号绑定与换路已交付 | 41项真实运行场景、既有回归及[PR CI](https://github.com/xinchen6666/xinapi/actions/runs/45690896769)通过；[Draft PR #8](https://github.com/xinchen6666/xinapi/pull/8)保留复审入口，见[X09日志](worklog/X09.md)与[E028](VALIDATION.md#e028-x09账号绑定及换路) |
| X10 | done | lead；严格出口已交付 | 331次实际请求观测、12项网关场景及124项既有回归通过；165次故障调用的上游／默认出口增量均零，已交付[Draft PR #9](https://github.com/xinchen6666/xinapi/pull/9)，见[X10日志](worklog/X10.md)与[E029](VALIDATION.md#e029-x10严格出口与故障停发) |
| X11 | done | lead；OAuth与只读回执已交付 | [原 PR #10](https://github.com/xinchen6666/xinapi/pull/10)记录 38 项运行、168 项三数据库检查与浏览器完成／取消。既有权限字段断言已在[主干收拢](worklog/X11-consolidation.md)按授权更新并复跑通过；历史证据见 [E030](VALIDATION.md#e030-x11管理员oauth与回执恢复) |
| X12 | done | lead；CPR 引擎／账号、Go BFF、Web | [PR #12](https://github.com/xinchen6666/xinapi/pull/12)经审阅修订以普通合并提交进入主干，主干双镜像成功；管理工作台实测见[工作日志](worklog/X12.md) |
| X13 | done | lead；成员授权、固定渠道与管理界面 | 从更新主干交付[PR #13](https://github.com/xinchen6666/xinapi/pull/13)，三数据库授组／撤权、WS 下一轮及浏览器验证通过，见[工作日志](worklog/X13.md) |
| X14 | in_progress | lead；模型转发、用量事实与联调 | X13 合入主干后启动 `task/X14-model-relay`，见[工作日志](worklog/X14.md) |
| X15 | pending | 跨服务测试 | 模拟矩阵／获授权的真实验收分别记账 |
| X16 | pending | 单机交付 | 从新克隆安装、首次管理员使用 |
| X17 | pending | 专项演练暂缓 | 等待单独确认维护范围和时间；X18 直接依赖 X15 与 X16 |
| X18 | pending | 版本发布 | 候选标签、接手试用、实际覆盖说明 |

`pending` 表示排队；`in_progress` 表示正在实施或交付；`blocked` 表示存在具体障碍；`done` 表示功能目标与该步 GitHub 交付均已核对。工作分支同步、合入 `main`、服务器部署分别记录。V01–V03 的最新覆盖见 E022，其余场景以各证据条目为准。已完成的 D01–D05 属于前期研究，证据见[验证记录](VALIDATION.md)，X 业务开发成果按本表记录。

定义与依赖见[开发计划](PLAN.md)，接手动作见[交接单](HANDOFF.md)。
