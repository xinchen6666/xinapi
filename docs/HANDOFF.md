# 当前交接

## 当前：X06 审核修复本地通过，待远端同步

用户已授权修复，R1–R6 和部分接受显式确认均已落实。当前复验为 passed：72 项真实接口场景、桌面/移动确认、账号与配置状态对比通过；Rust 完整回归 399 / 680 / 218 项、架构 16 项、严格 Clippy、Go 与前端检查均通过。详见 [修复报告](worklog/X06-fix.md)、[脱敏证据](evidence/X06-fix.json)和 [E022](VALIDATION.md#e022-x06审核修复复验)。

新合同：候选新增 parse_valid，supported 继续表示原生 HTTP/SOCKS 支持；下载目标逐跳校验全部地址并固定连接；HTTP 测试名单仅在明文 HTTP 开放本地例外。结果预算为 4096 候选、4096 诊断、完整 JSON 8 MiB，超额返回 subscription_preview_too_large。部分结果由管理员选择解析有效节点并确认范围，页面确认只保存在当前预览。

本地工作分支为 task/X06-subscription-preview，原 Draft PR #5 保持。Git 通道探测超时；代码和证据先形成同组本地提交，远端同步按实际回执另记。X06 保持 in_progress 以区分功能修复通过和远端交付完成。下一动作是同步本步提交与证据，再进入后续任务。

原 xinapi-x04-review 与 xinapi-x04-local 保持原镜像和数据。本轮验证使用 xinapi-x06-audit 与 xinapi/cpr:x06-fixed、xinapi/new-api:x06-fixed；私有脚本、配置、身份和构建日志保存在 .runtime/x06-fix 与原审核私有目录。临时服务收尾后停止，隔离数据卷保留供复跑。用户在[协作入口](../AGENTS.md)和[当前规则](MEMORY.md)的编辑继续保留。

本地与远端存在既有历史差异，后续同步应保留本地源码、截图及用户编辑，经审阅的合并或独立分支完成集成。

## 上一轮 X06 交付记录

原交接中的 X06 完成判断先由 E021 复核，当前功能修复结果见 E022；上一轮构建、接口演示和交付详情见 [X06 日志](worklog/X06.md)。

以下为 X03 历史交接材料。

## X03 历史交接
X03的CPR版本修订已完成。本轮交付统一入口的版本传递、CI版本输出比较及实际运行记录。X04保持pending，后续按用户确认的任务安排推进。

## 当前版本修订

统一入口从所选提交读取发行值3.54.2并传入镜像；本机重建与CI同一版本命令实际输出3.54.2，exit0。旧空版本镜像在新比较步骤返回exit1。现有47项工具回归通过，测试文件保持原样。

[PR CI 45120537811](https://github.com/xinchen6666/xinapi/actions/runs/45120537811)及[push CI 45120533870](https://github.com/xinchen6666/xinapi/actions/runs/45120533870)全部作业成功；两轮CPR日志均记录 `CPR release version verified: 3.54.2`。详细过程见[X03工作日志](worklog/X03.md#cpr-版本审查修订)和[E015](VALIDATION.md#e015-x03cpr-发行版本修订)。协作入口和记忆的既有本地编辑独立保留。


## 已验证成果

- 基线是X02已审查提交 `072e434a528bb9295d9c6d4ee6000c91b03485ca`；原services源码、逐文件manifest、依赖锁及许可文本未改。构建的根适配只控制同digest网络通道、资源、来源标签和许可证封装。
- New API、CPR镜像本地实际构建及无网只读短命CLI通过；CPR版本3.54.2，New API原VERSION空blob未伪造。镜像ID、artifact、平台和许可对照见[New API证据](evidence/X03-new-api.json)、[CPR证据](evidence/X03-cpr.json)。
- 提交 `a1da347925957c6f1a0ec50bd8157bb55457246a` 的[GitHub run 45104467732](https://github.com/xinchen6666/xinapi/actions/runs/45104467732)质量、两个镜像及汇总均成功，包含47项工具回归；严格绑定BuildKit导出、加载镜像、平台和源码标签。最终记录只改文档/证据，不重复宣称编译。
- 用法见[构建说明](BUILD.md)，实际过程/失败/取消/成功见[X03日志](worklog/X03.md)和[E014](VALIDATION.md#e014-x03实际镜像构建与持续检查)。源码树通过、可执行CLI和服务集成是不同证据，V01–V20未运行。

## 分支和接手

X03审阅入口为[PR #2](https://github.com/xinchen6666/xinapi/pull/2)，工作分支为 [task/X03-build-ci](https://github.com/xinchen6666/xinapi/tree/task/X03-build-ci)，依赖[PR #1](https://github.com/xinchen6666/xinapi/pull/1)。PR #2以X02分支为base，审阅范围聚焦本步构建与CI。合并顺序为先处理X02，再将X03调整至main，并依据实际PR状态开展后续工作。

进入X04前再查PR/main真实状态，从已验证的X03基线开始。X04将建立New API、CPR、PG18分库分用户、两个Redis的根Compose，默认只对本机发布New API。当前没有根Compose、真实账号、订阅或固定出口功能验收。

## 运行边界

本步没有部署业务、数据库或Mihomo，只有隔离短命命令；全部构建后台job已回收。本机既有容器未删除/重置。`.runtime/`保存临时工具与日志，不提交，真实GitHubtoken不进入Docker配置、build args、源码或CI。

Docker29的Id可能是OCI index而非config digest；不能机械把二者比较为同一值。历史来源验证用包含当前schema的X02提交，不用第一次缺import_tree字段的旧导入提交。APT安全更新/时间等可能影响字节，实际镜像用ID/digest识别，不保证跨日期逐字同构。
