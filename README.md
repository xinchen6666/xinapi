# xinapi

给朋友小团队私用的 Codex 订阅网关：管理员在 New API 后台导入 Clash 订阅、确认出口、授权上游账号并给成员分组。成员使用自己的平台 Key。**本仓库已准备固定上游源码及来源验证工具，还不是可启动的集成成品；实际进度见状态表。** 发布位置：[xinchen6666/xinapi](https://github.com/xinchen6666/xinapi)。

## 阅读入口

| 想做什么 | 去哪里 |
| --- | --- |
| 查看每一步如何开发、如何验证 | [Docker 开发计划 X01–X18](docs/PLAN.md) |
| 了解五个服务、首次安装和恢复设计 | [Docker 部署设计](docs/DEPLOYMENT.md) |
| 看现在完成了什么、推送到哪 | [任务状态](docs/STATUS.md)、[X02 工作日志](docs/worklog/X02.md)、[X03 构建日志](docs/worklog/X03.md) |
| 构建镜像和运行持续检查 | [构建入口与边界](docs/BUILD.md) |
| 换人／换 agent 继续 | [协作入口](AGENTS.md)、[当前交接](docs/HANDOFF.md) |
| 查产品范围和设计依据 | [项目需求](docs/PROJECT.md)、[架构](docs/ARCHITECTURE.md)、[决策](docs/DECISIONS.md) |
| 查长期约束、来源、许可证和证据 | [项目记忆](docs/MEMORY.md)、[固定上游清单](docs/UPSTREAM.md)、[验收记录](docs/VALIDATION.md) |

## 首版边界

只交付单机 Linux Docker Compose。New API 提供 React 管理界面和 Go 后端；内部 CPR（codex-proxy-rs）处理账号授权、刷新和模型请求，监督 Mihomo 子进程。PostgreSQL 18 分库分用户，两个应用各用独立 Redis。账号默认不得在代理故障后改走直连；公网 IP 能否真正独立取决于提供商和实际验证。没有支付、K8s 或 Helm 工作项。

第一个可见里程碑是 **X06：管理员粘贴自造的订阅链接能预览节点**，不必先提供真实账号。[New API 源码](services/new-api/README.md)、[CPR 源码](services/codex-proxy-rs/README.md)及[第三方来源索引](third_party/README.md)已随本仓库维护；正式构建依赖仍按各自锁文件下载，不是离线全依赖包。根 Compose 和集成业务在后续步骤交付。

## 研究材料

[订阅解析与固定出口](research/feasibility-subscription.md) · [统一管理员后台](research/feasibility-admin-integration.md) · [项目记忆方法](research/project-memory-methods.md) · [早期候选对比](research/subscription-gateway-research-2026-11-22.md)。这些报告是固定版本研究，不是本项目业务验收记录。研究用本地源码副本、真实订阅、凭据、日志和备份不会随仓库同步。
