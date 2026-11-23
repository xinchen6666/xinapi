# 当前交接

当前任务是 [X03 构建与CI](PLAN.md#x03)，用户已明确继续开发；X04及真实账号/服务部署没有开始。工作分支 `task/X03-build-ci` 基于已审查X02提交 `072e434a528bb9295d9c6d4ee6000c91b03485ca`，PR #1还未合并，不能把旧main当成含源码的基线。

## 本轮进展

New API 原正式构建遇到Docker Hub网络超时。验证可达registry返回的manifest字节与原digest相同后，使用同版本/同digest通道继续；Go下载也通过显式proxy适配，保留checksum/GOSUMDB。镜像构建及无网络短命CLI检查通过，来源标记、可执行文件和许可摘要已记录在[New API证据](evidence/X03-new-api.json)。

CPR原Dockerfile同样在Docker Hub阶段超时。相同digest通道的前端、UI prepack、PG备份工具已构建成功；Rust后端仍在依赖下载/构建，本步不能据此声称两个镜像均完成。实际结果由[本步工作日志](worklog/X03.md)收敛。

新增[构建说明](BUILD.md)、提交快照构建入口、项目文档/任务/凭据特征检查、路径分流与只读CI。44项项目工具回归和Ruff检查通过。CI尚需GitHub实际执行，工作流文件存在不等于检查通过。原始services、来源manifest和上游许可文本不改。

## 下一动作

1. 完成CPR后端和最终镜像的无网络CLI、静态资源、来源标签、许可检查。
2. 从已提交快照使用统一构建入口复验，推送X03分支，让GitHub独立执行质量与两个镜像作业；分别记账成功/失败/跳过。
3. X03验收后，再按授权进入X04。当前不自动合并PR、不发布业务端口、不接入真实OAuth/Clash订阅。

构建缓存和详细日志在忽略的`.runtime/`；公开只存脱敏命令与元数据。上游Git根与导入树的区别、历史校验器规则保留在[X02审查](worklog/X02-review.md)。旧本地平行历史备份不重推，不强推远端；最新branch/PR SHA用真实查询核对。
