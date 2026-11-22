# 当前交接

项目为朋友小团队非商业使用，采用单机Docker Compose。先读[状态](STATUS.md)、本页和[协作入口](../AGENTS.md)，再核对实际分支、GitHub PR和已验证提交。

## 当前成果

X02固定源码基线已交付到 [task/X02-source-baseline](https://github.com/xinchen6666/xinapi/tree/task/X02-source-baseline)，审阅入口为 [PR #1](https://github.com/xinchen6666/xinapi/pull/1)。两服务共4,914文件，来源清单、许可、Mihomo制品元数据及离线校验工具齐全。逐文件/模式/tree验证和11项工具测试通过；与远端已验证SHA相同的独立本地克隆也已通过，无研究缓存。记录见[X02日志](worklog/X02.md)和[E010–E012](VALIDATION.md#e010-x02固定源码导入与内容核验)。

X02完成表示任务验收并交付工作分支；不表示PR已合入main、CI通过或服务已部署。CI建立和应用构建是X03的任务，当前没有GitHub checks/status不能冒称通过，也不应成为X02源码交付反复重跑的理由。

## 下一步

进入[X03 基线构建与CI](PLAN.md#x03)前核对PR #1的审阅、合并状态和远端ref。若PR尚未合并，明确使用X02工作分支作为基线或先依仓库流程合并；不要从缺少源码的旧main启动构建。不得跳过用户授权去部署真实服务器或导入账号。

标准Git网络传输曾不稳定，X02使用过MCP文本提交。本地原有平行提交保存在 `backup/X02-local-before-sync-27fe3a2a1100`。已按远端commit SHA和相同tree校验后对齐工作分支，不再对备份分支重复推送、重放历史或强推。最新head以实际Git/PR查询为准，不为文档自引用不断增加提交。

## 固定边界

- New API upstream/import tree：`11dbbb8a51210a38b84d9f9358c56c9d0415f5e8`。
- CPR上游发布tree：`d606f1081ffdeb53dbbd9efd89c6edef9427887e`；省略两个可选源码联调gitlink后的普通文件import tree：`3edc9042af879db4d0d8c5a09c39d2b142f5994c`。这不是文件丢失，也不应将gitlink重新混入父仓库。
- 后续修改源码后，基线校验使用已确认的X02提交，不覆盖原始来源哈希。正式包依赖仍要按锁文件下载，X02不是离线依赖包。
- Cockpit未复制业务实现；混合许可边界仍待实际采用时确认。Mihomo `.gz`摘要来自官方release元数据；未下载、解压、验签或执行。
- 还没有两服务业务构建、根五服务Compose、真实OAuth或固定出口验收。工作记录、模拟数据和未执行项不能替代运行证据。

凭据不写入源码、日志、PR或交接。当前没有本项目业务容器、数据库或真实账号流量。
