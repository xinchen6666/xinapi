# 当前交接

## 当前：X06 已关闭，下一步 X07 快照与刷新

X06「订阅粘贴与预览」已完成：CPR 新增 `POST /api/admin/subscriptions/preview`（Clash YAML、Clash JSON、URI 列表与 Base64 文档，`proxy-providers` 受控物化一层，正文 2 MiB、解压后 8 MiB、10 秒预算、跳转上限 3 且逐跳复核，明文 http 仅对 `subscriptions.insecure_test_hosts` 内主机开放）；New API 新增 `POST /api/account-egress/subscription-preview`，沿用 AdminAuth 与 `account_egress:read`，响应只含白名单字段；前端新增 `/subscription-preview` 页面与七语文案。验证见 [X06 日志](worklog/X06.md)与 [E020](VALIDATION.md#e020-x06订阅粘贴与预览)。

交付：本地提交 `3ad1ee5`；远端分支 `task/X06-subscription-preview` 经 GitHub 内容接口送达，读回的分支头与文件清单和本地提交一致；交付形态为 [Draft PR #5](https://github.com/xinchen6666/xinapi/pull/5)，base 为 `task/X05-admin-readonly`，保持 open。四张页面截图留在本地 `docs/evidence/`，随网络恢复后的普通推送进入远端。

网络边界与对齐动作：本机到 github.com 的 HTTPS 通道当前中断，`git push` 以超时结束；GitHub 内容接口与 api.github.com 可用。网络恢复后依次执行 `git fetch origin` 与 `git reset --hard origin/task/X06-subscription-preview`，随后重跑 `git show --stat` 核对工作树与远端一致。

下一步 X07：快照、刷新与迁移——在预览合同之上建立来源、成功快照、节点 UUID 与不可变连接版本，覆盖改名保持绑定、拉取失败保留上次成功配置与并发刷新不覆盖新批准值。

运行环境：本机 `xinapi-x04-review` 保持真实 CPR 五服务，New API 回环端口 19304，镜像回到 `xinapi/new-api:x05-reviewed` 与 `xinapi/cpr:x03-version-fix`；测试用 `xinapi/new-api:x06-preview` 与 `xinapi/cpr:x06-preview` 保留在同名标签下供复跑。原 `xinapi-x04-local` 回环端口 18304 保持运行。Go 测试使用 `golang:1.26.10-alpine` 容器、卷 `xinapi-gomod` 与 `GOPROXY=https://goproxy.cn,direct`；Rust 构建使用 `rust:1.99.1-bookworm` 与卷 `xinapi-cargo`，磁盘在构建前留出 10 GB 以上余量。[协作入口](../AGENTS.md)及[当前规则](MEMORY.md)的用户编辑继续独立保留。

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
