# 当前交接

## 当前：X04 已交付

五服务运行方式见[本机手册](COMPOSE.md)，实测与限制见[X04 日志](worklog/X04.md)。X04 分支依赖 X03 PR #2。X05 将开发 New API 管理员只读入口及服务端权限检查；真实账号、订阅和固定出口仍属后续任务。

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
