# X02–X11 主干收拢

| 字段 | 内容 |
| --- | --- |
| 负责人 | lead：集成与交付；管理权限与远端检查分域实现 |
| 开工时间 | 2026-11-26，Asia/Shanghai |
| 前置 | X02–X11 串行 PR 栈，X11 最终 PR CI 已成功 |
| 工作分支 | `integration/x02-x11`，沿用 X11 已审阅的提交历史 |
| 用户确认范围 | 独立归档规则与脱敏资源报告；管理写入、外部订阅预览统一权限／会话／同源；仅更新指定权限用例期望；精简路线与日常 CI；通过普通合并提交合入 `main` |
| 交付判断 | 指定权限用例通过；汇总 PR 与主干 CI 查到实际结论；旧 PR 收尾，任务分支继续保留 |

## 执行记录

- 已从 X11 建立收拢分支，三份规则与资源材料作为独立提交归档。资源报告保留历史时点，宿主管理地址细节已脱敏。
- 原分支历史保留，X06 的独有文档和 X02 本地保留分支维持原样。X12 页面、节点验证接口和 X16 安装代码留给对应阶段。

## 实际验证与交接

- Go 1.26.10：`go test -mod=readonly -p 2 ./service/authz -run '^TestSetUserPermissionsStoresOnlyOverrides$' -count=1 -v` 通过；会话 Origin、PAT 路由完整性与权限目录回归均通过。正式测试仅更新获准用例中的两处账号出口字段期望。
- 项目规则工具 15 项既有回归、工作流语法、Compose 配置解析通过。
- GitHub 验证入口：[收拢 PR 查询](https://github.com/xinchen6666/xinapi/pulls?q=is%3Apr+head%3Aintegration%2Fx02-x11) · [主干 Actions](https://github.com/xinchen6666/xinapi/actions/workflows/x03.yml?query=branch%3Amain)。镜像构建与实际部署分开记录。
- 权限边界参考 [OWASP ASVS 5.0.0](https://github.com/OWASP/ASVS/releases/tag/v5.0.0_release) 的会话、授权及浏览器来源校验要求。真实第三方授权和完整生产部署仍待对应阶段验证。
