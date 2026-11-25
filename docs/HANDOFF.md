# 当前交接

## 当前：X07 来源与批准版本首个增量已交付

已实现来源创建/编辑/读取、受控刷新差异、显式选择身份和批准版本。Admin 以规范连接字段直接匹配，Store 单点 AES-256-GCM 密文保存来源与连接；稳定节点 UUID 与只追加连接版本分离，改名/排序/等价默认值保留身份。完整缺失需确认后标记 available=false，部分结果保留旧节点；旧刷新/旧批准受 revision 和 review_id 校验。

New API 复用「订阅预览」页提供「来源管理」：管理员只读；超级管理员登录会话执行写入，个人访问令牌仅保留读取职责。URL只作为提交字段，列表与详情保持元数据投影。密钥在 CPR 运行数据卷内 `subscription.key`（0600、32字节），已有数据时缺少或错误密钥拒绝读写。

实际结果：15项业务流程、6项补充、真实桌面/移动、密钥、重启与迁移场景通过；Rust订阅25项/API7项/架构16项、Go权限路由、前端构建和类型检查通过。整仓前端56项失败/超时和controller整包超时分别保留。范围及截图见 [X07日志](worklog/X07.md)与 [E023](VALIDATION.md#e023-x07来源与批准版本)。

当前分支 task/X07-subscription-snapshots，首个增量以 [Draft PR #6](https://github.com/xinchen6666/xinapi/pull/6)交付，base 为 task/X06-subscription-preview。完整55个源码、文档和截图文件经官方 GitHub 接口送达，工作日志及截图已回读；Git传输低速后使用本机受信凭据完成官方接口提交。X07保持in_progress，下一动作是专项复审来源身份、批准语义和密钥生命周期；账号绑定、实际出口及引擎接入依既定后续任务推进。

原 X04 运行栈保留。验证使用独立 xinapi-x07-dev 与 xinapi-x07-upgrade；私有配置、身份、源码验证工具及日志位于 .runtime/x07 与 .runtime/x07-upgrade，临时开发、迁移和密钥验证服务已停止，隔离数据卷继续保留。协作入口和长期记忆的用户编辑继续独立保留。

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
