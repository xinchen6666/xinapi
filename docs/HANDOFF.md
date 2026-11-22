# 当前交接

项目为朋友小团队非商业使用，采用单机Docker Compose。先读[状态](STATUS.md)、本页和[协作入口](../AGENTS.md)，再核对本机分支、PR与远端。

## 本轮要求与成果

用户明确暂停X03，先审查已交付X02，有问题修复并推送同一个仓库。本轮只审查源码基线、来源校验器、测试、元数据和交接记录，不运行或修改上游业务。

X02初次交付见[X02日志](worklog/X02.md)；这次发现并修复两处校验工具的错误通过：工作树服务根/父目录可被外部符号链接替换；Git中的provenance模式被忽略、symlink JSON可被当作普通manifest。新增回归后共16项，通过正常来源校验，实际4,914个上游文件仍未改动。审查、复现与修订证据见[X02审查日志](worklog/X02-review.md)、[E013](VALIDATION.md#e013-x02审查修订)。另把历史校验示例固定到包含当前schema的已验证提交，避免误选早期缺字段版本。

审查修订继续在 [task/X02-source-baseline](https://github.com/xinchen6666/xinapi/tree/task/X02-source-baseline) 和 [PR #1](https://github.com/xinchen6666/xinapi/pull/1) 交付。最新head用GitHub及本地ref实时核对，不为文档自引用重复加提交。PR未合并；本轮没有部署。

## 下一步

本轮仅完成X02审查与修订，不进入X03。后续收到继续开发指令，再确认PR审阅/合并状态及基线，进入[X03构建与CI](PLAN.md#x03)。CI暂无结果不能写成通过，也不应代替本步已执行的工具测试；不要未经指令部署或接入真实账号。

## 必须记住

- New API import/upstream tree：`11dbbb8a51210a38b84d9f9358c56c9d0415f5e8`。
- CPR upstream tree：`d606f1081ffdeb53dbbd9efd89c6edef9427887e`；省略两个可选联调gitlink后的普通文件import tree：`3edc9042af879db4d0d8c5a09c39d2b142f5994c`。源码原样保留，不能通过改manifest掩盖业务修改。
- 用当前schema复查历史基线的例子：`python3 -B tools/verify_upstream.py --git-ref dc5263120fd97ee92ddc39e947b55b4ee3bc9a82`。最早`5b93f04…`缺import_tree字段，不是该校验器的同schema目标。
- worktree必须是仓库内真实目录，manifest必须普通100644文件。index／historical只读被选择的Git对象，不依赖当前工作树是否改变。
- Cockpit未复制业务代码；Mihomo只有制品metadata和许可，未下载或运行。两服务的业务构建、OAuth、Clash节点和实际出口测试仍未执行。

原本地平行提交在 `backup/X02-local-before-sync-27fe3a2a1100` 保留；不要重复推送备份、强推或重做已经完成的源码导入。当前没有本项目业务容器、数据库或真实账号流量。
