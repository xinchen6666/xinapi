# X16 真实单机部署与 ECH 兼容

- 开工：2026-11-28（Asia/Shanghai）；负责人：lead 部署与集成，独立代理核对官方 ECH 合同和源码差异
- 授权：用户指定目标主机、域名、私有订阅及 5 人规模，随后明确选择补齐 ECH 兼容并继续部署
- 前置：[PR #19](https://github.com/xinchen6666/xinapi/pull/19)已普通合入主干，[主干构建](https://github.com/xinchen6666/xinapi/actions/runs/46034381508)通过
- 写入：Admin 节点规范化、Host ECH 字段和固定下载客户端声明，以及本轮脱敏部署说明。正式测试文件、持久层结构和原工作树编辑保留

## 部署与兼容合同

目标为 Ubuntu 24.04 x86_64、约 2 GiB 内存和 30 GiB 磁盘，已有独立应用及 Caddy。新增独立五服务项目、私有配置与数据卷，团队入口复用 Caddy 的独立 HTTPS 站点；New API 仅绑定宿主回环，其余四服务仅容器网络可达。目标 CPU／内存资源用于运行，开发机串行完成 Go 与优化 Rust 构建；现有受控运行层复用，替换同源新编译程序，前端来自当前锁定源码构建。

新实例按既有小内存指南设置 PostgreSQL 连接和查询预算、CPR 五连接池、New API 八连接池和独立容器内存限制，受管引擎保持两槽。5 人属于计划成员规模，真实并发和上游容量仍需账号授权后验证。

用户订阅提供五个 VLESS TLS／WS 节点和一个 AnyTLS 节点。实际节点开启 ECH，并带导出器默认字段；Admin 将 VLESS 空 flow、alterId 0 和 cipher auto 归一，保留 ECH enable/config/query-server-name。Host 仅向 VLESS TLS／WS 放行标准 Base64 内联 ECH，保持其原值、客户端指纹和证书验证；缺少内联配置、字段错误及范围外组合返回既有诊断，AnyTLS 继续标为范围外。

订阅服务默认返回 Base64 URI，ECH 只带开关；声明 `clash.meta (xinapi)` 时返回完整 Clash 节点文档。因此下载源使用固定客户端标识协商结构化内容，公网目标限制、逐跳 DNS 固定、凭据剥离、解压和时间预算保持原样。

依据：[Mihomo ECH options](https://github.com/MetaCubeX/mihomo/blob/v1.19.33/adapter/outbound/ech.go)、[VLESS](https://github.com/MetaCubeX/mihomo/blob/v1.19.33/adapter/outbound/vless.go)、[WS](https://github.com/MetaCubeX/mihomo/blob/v1.19.33/transport/vmess/websocket.go)、[所锁 TLS 库合同](https://github.com/MetaCubeX/utls/blob/v1.8.8/common.go)。内联配置优先于 DNS 查询，握手失败直接返回，保持 ECH 开启语义。

## 实际结果

- 公共 DNS 地址记录生效，Caddy 自动取得受信证书；外部 HTTPS 返回 200
- 管理员在回环入口完成初始化后才开放站点，公开注册关闭；真实浏览器登录及 Secure／HttpOnly／SameSite=Strict Cookie 通过
- 两个原有站点继续正常响应，独立数据库和 Redis 启动成功
- New API 类型检查、前端构建、Go 生产编译、CPR release 优化编译、全量格式及 Admin／Host 库严格 Clippy 通过；一次性工具直接编译生产模块，核对 ECH／TLS／WS 保留、等价默认值和错误配置拒绝，三类行为通过
- 真实订阅经固定客户端协商返回 Clash 文档，五个 VLESS 节点解析并明确批准，一个 AnyTLS 按范围外跳过；五条实际连接全部通过，观测耗时依次约 1316／1292／1101／1170／192 毫秒，ECH 与证书校验保持开启。订阅含范围外节点，因此审批按明确部分接受执行
- 已安装应用、证书、管理员、Team 组和私有管理 Key；公开站点可登录。账号持有人尚待完成官方 OAuth，实际模型目录、专用渠道、成员授组和首个模型请求随授权后继续，节点连接通过仅代表当前公网出口可达
- 独立源码复核通过，原工作树编辑保留；脱敏结果见[部署证据](../evidence/X16-live.json)

真实域名、订阅 URL、节点地址及认证、账号密码、管理 Key 和出口 IP 均仅存于受保护部署环境，公开文档仅记录结果。
