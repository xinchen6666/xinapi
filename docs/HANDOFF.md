# 当前交接

2026-11-22 UTC · 模式：小团队非商业使用，**只交付单机 Docker Compose**。先读[协作入口](../AGENTS.md)、[进度表](STATUS.md)和本页，确认 `git status --short --branch`、`git remote get-url origin`、当前提交／远端头，再选择任务。

## 当前工作与下一动作

已用 X01–X18 原地替换旧路线。公开文档基线已在本地提交为 `5b310228174ada93e1006c7f97c05e1f575295db`；此后另有记录推送结果的文档提交，用 `git rev-parse main` 取得最新 SHA。对 [xinchen6666/xinapi](https://github.com/xinchen6666/xinapi) 的 HTTPS 推送因本机未提供 GitHub 登录凭据而失败；只读复查 `refs/heads/main` 不存在。准确记录见[执行日志](worklog/X01.md)。没有启动业务服务或导入真实上游账号。

**下一动作是同步已有提交，不是重新初始化、重新建仓或重新生成计划。**所有者先在本机通过受信的 HTTPS 凭据管理器（或已验证主机密钥的 SSH 配置）完成 GitHub 授权，授予该仓库写权限。不要在聊天、Git URL 或文档里传令牌。检查 `git remote get-url origin` 及 `git ls-remote --heads origin`：仍为空或兼容快进时执行 `git push -u origin main`；若远端已进新历史，先 fetch、检查并保留用户提交，不强推。比较 `git rev-parse main` 与 `git ls-remote origin refs/heads/main` 的完整 SHA，记录回执后再关闭 X01。

之后是 [X02 固定源码与许可](PLAN.md#x02)，不是直接部署。X03 才开始基线构建，X06 才是能在管理界面看到测试节点的演示。当前无在途部署任务、服务或真实账号测试。

## 不要误判

- 旧报告是调研快照；旧 P 编号只通过[历史对照](PLAN.md#legacy)解释，不再执行旧计划。
- CPR 绑定失效时可能走默认网络，账号停用不等于后台 OAuth 续期停发。稳定本地端口换节点不会自动使旧 HTTP/2 和 WS 连接消失。
- CPR 的管理 Key 是共享服务身份；New API 需保存本地授权操作的管理员归属。只读 OAuth 完成回执是 X11 的新增功能，当前不能假设已有公开接口，也不能从账号列表猜测成功。
- 普通新成员未授组前不得使用上游。每成员／授权范围的 CPR 内部 Key、平台成员 Key、上游 OAuth 令牌和 CPR 管理 Key 是四种不同凭据。
- 用户的仓库是源码目标，推送代码不授予自动部署权限。完整的订阅链接、凭据、运行日志及原始研究缓存禁止公开。

收工时用当前事实替换本页：任务 ID、最新本地提交、远端是否同步、未完成验证、下一动作和仍活跃任务。长期事实进[项目记忆](MEMORY.md)，证据进[验证记录](VALIDATION.md)，不依赖上一位 agent 的聊天。
