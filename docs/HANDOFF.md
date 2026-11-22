# 当前交接

2026-11-22 UTC · 小团队非商业，单机Docker Compose。新会话先核对[状态](STATUS.md)、本页及`git status --short --branch`，再根据目标branch和日志判断下一动作。

## X02

X01已在`main`。X02固定上游源码已导入 `task/X02-source-baseline`；实际工作记录见[X02日志](worklog/X02.md)，计划见[PLAN X02](PLAN.md#x02)。

New API固定于`8542c9d6490b4330fe18160930f81ae7428bc777`：源tree及导入tree均为`11dbbb8a51210a38b84d9f9358c56c9d0415f5e8`。CPR固定于`58cbb607505dda4a1cee6e01b9f76e267ad3c261`：上游发布tree为`d606f1081ffdeb53dbbd9efd89c6edef9427887e`，包含两个可选联调gitlink；它们未作为嵌套repo导入，2,074个普通文件的导入tree为`3edc9042af879db4d0d8c5a09c39d2b142f5994c`。四个来源SHA不能混写。两个源manifest由`tools/verify_upstream.py`分别验证，结果尚待干净clone及GitHub远端校验。

Cockpit的CC BY-NC-SA组合尚未确认，本步没有复制其业务代码；Mihomo只锁官方发布metadata和许可证，二进制尚未下载/执行。两服务源码、Python来源校验工具和11项回归测试已在工作区；需要按本步日志核查实际index和分支SHA。

## 接下来

1. 最终校验worktree及index中的两个不同CPR tree：import tree `3edc...`，upstream tree `d606...`；运行11项测试。E010/E011重复已在VALIDATION收敛。
2. 核对公开暂存内容、许可文本和无意外gitlink。用最新X02 commit做干净clone，不使用本机研究缓存，运行两个来源校验和工具测试；不能拿旧中间提交的clone结果代替。
3. 推送`task/X02-source-baseline`并从GitHub读回准确SHA，创建PR到`main`，检查CI。PR未合并前X02是`in_progress`；不自动合并、不部署。
4. 合入后记录远端main合并SHA并置X02 `done`，下一任务X03基线构建。

本步不得构建/运行上游应用、安装依赖、启动Docker、下载或执行Mihomo、导入账号/订阅。管理凭据由受保护环境提供，不写日志。当前无允许真实部署的主机。

## 后续错误边界

PR中应明确：Mihomo压缩asset metadata未实测；New API上游可能含合成示例凭据；下载与构建依赖仍待X03。CPR UI和plugins源码gitlink不是导入目录，但其正式依赖分别有锁定提交；若clean-clone构建要求预打包，X03再判定是否需要新增component source，不要把全量研究checkout或gitlink直接混入。
