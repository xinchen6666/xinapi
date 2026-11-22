# 项目协作入口

## 从新会话恢复

1. 确认项目根目录、当前分支、`git status --short --branch` 及实际远端；仓库目标为 [xinchen6666/xinapi](https://github.com/xinchen6666/xinapi)。研究子目录的 Git 不是本项目根仓库。
2. 读[进度](docs/STATUS.md)、[交接](docs/HANDOFF.md)。用当前任务 ID 查[唯一计划](docs/PLAN.md)，从本步工作日志、验证索引和远端 SHA 核实状态。不要假定上个 agent 的后台任务或聊天可用。
3. 按需要读取[需求](docs/PROJECT.md)、[架构](docs/ARCHITECTURE.md)、[部署边界](docs/DEPLOYMENT.md)、[长期记忆](docs/MEMORY.md)、[上游](docs/UPSTREAM.md)。进入实际修改的上游模块时还要遵循适用的局部约定。

## 实施纪律

- 用户范围：小团队非商业；单机 Docker Compose；New API 是身份和页面入口，只有管理员处理节点订阅、上游账号和出口。发布源码不等于部署服务。未获真实账号和服务器授权时用自造数据或标记 `not_run`。
- 一个任务一个永久 ID，采用 [X01–X18](docs/PLAN.md)，不要重新启用历史 P0–P5。开工写 owner、写入范围、依赖、验收；中断也写工作日志。获准的 X01 不代表后面任务已有实现。
- 每步用[模板](docs/worklog/TEMPLATE.md)建 `docs/worklog/Xnn.md`，代码、测试、迁移、用户说明和本步记录在同一组提交中。文档资产和失败证据也需要真实验证。不得只在宿主临时看板记录完成。
- 集成人员检查最终 diff 和实际运行结果；更新[验证](docs/VALIDATION.md)、[状态](docs/STATUS.md)和[交接](docs/HANDOFF.md)。状态为 `pending/in_progress/blocked/done`。状态表唯一；`done` 需要满足验收并核对 GitHub 交付。证据记录指明模拟/真实、版本、结果、未覆盖和失效条件。
- 改变长期事实才写[记忆](docs/MEMORY.md)；改变技术方向写[决策](docs/DECISIONS.md)，保留决定者、理由及替代关系。只有一个 owner 写全局状态、决策和交接。
- 并行任务规定互不重叠的写文件范围和前置合同。共享 DTO、迁移编号、锁文件和代码生成由集成人员协调。不要用批量格式化覆盖其他人的工作。

## 源码和 GitHub

X01 用空远端建立 `main`；之后用 `task/Xnn-简述`，提交例如 `feat(accounts): X11 add owner-bound receipts`。每步推送到指定仓库并比对远端分支 SHA；按仓库规则用 PR 合并到 `main`。分别记录本地提交、远端分支是否确认和是否已合入，不假定远端请求一定成功。不创建另一个远端、不强推、不为了收尾重写他人历史。

公开提交使用白名单审查；禁止真实订阅 URL、节点认证、OAuth code/state/AT/RT、管理 Key、`.env`、GitHub 凭据、运行日志或数据库备份进入 Git、PR、CI 输出或截图。日志只记脱敏结果和配置职责。不要公开真实出口 IP 或成员身份。`research/checkouts/` 是可选本地研究缓存，不提交；实际修改的产品源码必须在本仓库的 `services/` 受控树，而不是嵌套仓库指针。保留许可、来源与修改说明。

网络失败时保留本地 SHA；重试前检查远端 ref。没有写权限，通过本机受信凭据管理器等配置授权，不向聊天索取密码、PAT 或私钥。收工时记录准确的下一动作、工作树差异、未完成验证与运行任务；没有则写明。
