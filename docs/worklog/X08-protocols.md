# X08 四类节点协议执行扩展

- 开工：2026-11-27（Asia/Shanghai）；负责人：lead 集成、Host 协议配置与真实协议服务端分域处理。
- 前置：用户选择主要使用 VLESS／Trojan／Shadowsocks／VMess，授权继续。前序 [PR #18](https://github.com/xinchen6666/xinapi/pull/18)已普通合入主干，[主干镜像检查](https://github.com/xinchen6666/xinapi/actions/runs/45986245091)通过。
- 分支：`task/X08-protocol-support`，独立目录复用已审阅的源码；远端分支以当前 `main` 为基底，原首页与维护编辑保留。
- 范围：沿现有订阅规范化与批准版本，将四类协议的明确字段生成 Mihomo v1.19.33 固定节点配置。TLS 证书和传输组合按官方源码与实际握手核对，公开管理响应继续隐藏连接秘密。
- 写入：订阅协议能力、Host 配置生成、支持范围及脱敏实测记录；用户于 2026-11-27（Asia/Shanghai）明确授权同步两份现有测试文件内 5 项协议预期。持久层结构、账号绑定和发送准入保持原样。
- 验证：真实本机协议服务、私有 TLS CA、实际 CPR／Mihomo／New API 和受控模型上游；按协议执行导入、审批、节点连接验证、账号模型请求和故障停发。公网订阅和账号留待专项实测。

## 实施与结果

- 依据官方 Mihomo v1.19.33 [四协议出站](https://github.com/MetaCubeX/mihomo/tree/v1.19.33/adapter/outbound)、[TLS](https://github.com/MetaCubeX/mihomo/blob/v1.19.33/transport/vmess/tls.go)、[WebSocket](https://github.com/MetaCubeX/mihomo/blob/v1.19.33/transport/vmess/websocket.go)与[服务端入口](https://github.com/MetaCubeX/mihomo/tree/v1.19.33/listener/inbound)核对认证、SNI、ALPN 和连接字段。Host 的新叶子模块只处理四类协议，HTTP／SOCKS 既有路径保持。
- 来源能力覆盖六类协议族；保留规范化字段的白名单，节点验证时核对具体选项。SS 覆盖 AES-128-GCM／AES-256-GCM／ChaCha20-IETF-Poly1305；VMess 使用 AEAD、alterId 0，TCP／WS 可配 TLS；VLESS 和 Trojan 为 TLS TCP／WS。详细合同见[API](../../services/codex-proxy-rs/docs/api.md#15-受管引擎入口)。
- 四个真实协议服务端使用独立私网地址，10 个组合均完成实际连接验证和 JSON／SSE／WSS 模型调用，共 30 次成功生成。SS／VMess／VLESS／Trojan 各完成一次受控 OAuth 交换；目标 socket 分别观测 `.20`／`.21`／`.22`／`.23`，全部属于隔离网络。四协议服务端依次停止后，上游生成增量为零；错误认证、错误证书主机名和额外 dialer-proxy 选项在目标前拒绝。
- Clash JSON／YAML 和完整 URI 各接受 10 个组合，主四协议 Base64 URI 包装通过；公开响应仅保留节点元数据。四协议内核配置和 TLS 证书校验均真实执行。受管子进程清空继承环境，私有 CA 因此合并到容器系统信任位置，运行环境保护保持原样。
- Rust 1.99.1 锁定离线构建、全量格式、Admin／Host 库严格 Clippy 和 16 项架构用例通过。初次订阅/API 回归有 5 项仍固定旧协议预期；获用户明确授权后，仅同步两份现有文件中的这 5 项。API 字段白名单用例加入范围外节点以保留诊断字段校验，权限、敏感信息过滤和错误场景保持。最终 Admin 订阅 16 项、API 订阅 7 项全部通过，格式复查通过。
- 实际完整证据见[结构化结果](../evidence/X08-protocols.json)。初次 TLS 失败定位到夹具 CA 安装位置，初次重授权请求携带了新授权专用字段，均在外置验证器修正后复跑。独立源码复核确认固定节点、字段拒绝、TLS 及历史批准语义。公开发行、公网账号／节点、Reality／Vision／gRPC 与其他平台留待专项确认。
