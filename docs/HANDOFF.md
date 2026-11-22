# 当前交接

小团队非商业使用，单机Docker Compose。接手先读[协作入口](../AGENTS.md)、[状态](STATUS.md)和本页，再核对本机分支与远端。

## X02 当前发布

X02实际源码、来源清单、验证器和回归测试已推到[工作分支 `task/X02-source-baseline`](https://github.com/xinchen6666/xinapi/tree/task/X02-source-baseline)，处于[Draft PR #1](https://github.com/xinchen6666/xinapi/pull/1)，base `main`。GitHub当前PR head commit `22feb1265e76b2d3c4c9629fae2b55e4d7f2d944`：远端root tree `01129bf09e94ed3e6ca1047b895ed472e5c5a780` 与本地X02内容tree一致；New API与CPR的服务子树逐SHA等于 import tree。该commit SHA由GitHub MCP生成，和本地早期commit metadata SHA不同，但文件树一致。

Draft PR尚未合并，且截至记录时没有GitHub status/check-run。X02继续保持`in_progress`；等待维护者审阅和所需CI，不合并、不部署。PR详情与验证见[X02工作日志](worklog/X02.md)、[E010–E012](VALIDATION.md#e010-x02固定源码导入与内容核验)。

## 下一步

先让维护者审阅PR #1并确认CI/许可意见。合并到`main`后核对实际merge SHA，才把X02设为done，再基于合并后的main启动X03基线构建。真实New API/CPR服务、依赖、Mihomo、账号和Clash订阅都还没运行；不要从本地上游源码“存在”推断系统可用。

## 保留的技术边界

- New API来源tree/import tree `11dbbb8a51210a38b84d9f9358c56c9d0415f5e8`。
- CPR发布根tree `d606f1081ffdeb53dbbd9efd89c6edef9427887e` 含两个可选dev gitlink；xinapi导入tree `3edc9042af879db4d0d8c5a09c39d2b142f5994c` 只含普通文件。两个SHA分别核对，不能混写。
- Cockpit CC BY-NC-SA与Apache/AGPL组合许可尚未厘清，本步未复制其业务代码。Mihomo只记录release压缩资产摘要和GPL，未下载、解压、执行。
- 没有X01已同步分支的替换。PR branch上的代码不代表main已合并，推送不代表Docker已部署。
