# X11 双方式账号接入与授权引导

- 开工：2026-11-28（Asia/Shanghai）；负责人：lead 集成、前后端、隔离验证和部署
- 用户要求参考 ChatGPT2API 项目的账号接入方式，明确选择同时保留官方登录与 Token 导入
- 写入范围：OAuth 回执投影、受管 Token 接入、New API 会话适配和接入页面、七语文案；正式测试文件、既有数据结构及线上账号保留

## 原因与方案

当前官方链接仅在首次响应与弹窗内存中存在，关闭后 GET 回执缺少链接；已取消状态又在过期判断中变成 unknown。两项表现会让用户误以为仍待授权。GET 现在从原操作读取有效待登录链接，继续校验登录会话和完整冻结字段；列表保持普通回执，链接详情禁止缓存。取消和失败状态优先于到期，待登录到期显示 expired，账号 ID 仅在完成后显示。

参考项目 [v0.6.3](https://github.com/yang0/chatgpt2api-dsh/tree/8ad40e35cc4b759b056afbaa354bba62dd1ee18b)以 Access Token 导入和后端凭据自动登录为入口。用户选择的本轮范围为官方浏览器登录与直接 Token 导入，官方流程保留 state／PKCE／单次交换；Token 采用已有身份字段解析和真实上游访问验证，通过批准节点准备，再复用 CPR 既有授权提交以原子写入账号、代理与固定路由。令牌只存在请求和既有凭据持久化路径，New API 回执仅保存操作归属和结果。

页面复用 Dialog、CopyButton、Field、Textarea 与批准节点选择器，官方登录提供打开链接、官方登录和回调提交三步；Token 导入明确说明从本人 ChatGPT 会话页取得 accessToken，并在提交后清空。已有 Token 按自身有效期工作，后续可使用官方重新授权取得自动续期凭据。

## 相关鉴权边界

依据 OWASP [Authentication](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)、[Session Management](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html)、[OAuth](https://cheatsheetseries.owasp.org/cheatsheets/OAuth2_Cheat_Sheet.html)与[CSRF](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html)指南核对：当前管理员会话、对应操作权限、同源写入、服务端 state／PKCE、回调单次领取、有限时效、敏感请求体清理、回执零 Token 和固定出口。密码／2FA 自动登录及参考实现的 TLS 关闭方式保持本轮范围外。

## 验证

- 前端类型检查、限定 lint、生产构建，Go vet／构建，Rust fmt、Admin／Store／OpenAI Provider／API 严格 Clippy 和优化 release 构建通过
- 真实隔离五服务验证：GET 重开保持同一有效链接且零交换；取消后到期保留 cancelled；有效期结束返回 expired 并隐藏链接；官方回调重复提交保持一次交换
- Token 成功导入经已批准节点验证并原子建立账号／专属路由；错误与过期形状失败时零账号、零残留槽；导入账号 JSON／SSE／WSS 完成，停止入口后目标生成增量零
- 会话隔离、匿名拒绝、跨站写入拒绝通过；一次性调用现有 model 验证 SQLite／MySQL／PostgreSQL 的回执创建、保存、revision 条件、会话归属与撤销拒绝，三种数据库均通过
- 真实 Chromium 检查官方链接关闭／重开／页面刷新、明确取消状态、Token 表单、提交后清空及失败提示；截图为自造数据：[官方登录](../evidence/account-official-login.png)、[Token 导入](../evidence/account-token-import.png)
- 独立源码复核通过。新镜像部署到已授权站点，五服务正常；原两条已取消记录经实际服务读取仍为 cancelled，原账号及其他站点保留。线上只验证入口与表单，正式上游登录和 Token 由用户本人私有提交

运行结果和边界见[结构化证据](../evidence/X11-account-access.json)。
