# 上游清单与采用边界

更新：2026-11-22 UTC。下列提交是本轮证据基线，不代表运行中的部署，也不承诺未来 main 仍有相同行为。升级必须重新核对受影响接口与许可。

| 组件 | 固定基线 | 可选研究缓存（非仓库依赖） | 采用目的 |
| --- | --- | --- | --- |
| [New API](https://github.com/QuantumNous/new-api) | `8542c9d6490b4330fe18160930f81ae7428bc777` | `research/checkouts/new-api`，独立 Git | 统一 React 页面、Go 身份入口、成员 Key、配额和日志 |
| [codex-proxy-rs](https://github.com/zyycn/codex-proxy-rs) | `58cbb607505dda4a1cee6e01b9f76e267ad3c261` | `research/checkouts/codex-proxy-rs-58cbb607505dda4a1cee6e01b9f76e267ad3c261`，固定提交源码归档 | Rust账号池、OAuth、协议转发和标准账号代理 |
| [Cockpit Tools](https://github.com/jlcodes99/cockpit-tools) | `c004bf71be250cc2536fc91ef90638837f0013a0` | `research/checkouts/cockpit-tools`，Git及按commit补充的读取材料 | 订阅、节点、引擎和稳定入口的选择性抽离参考 |
| [Mihomo](https://github.com/MetaCubeX/mihomo) | Cockpit基线使用 `v1.19.33` | 本轮未安装二进制、未建立运行实例 | 外部网络代理引擎，后续固定平台制品与校验和 |

已有研究副本可能包含未完整 checkout 的历史获取尝试，不能仅凭 `.git/HEAD` 认定每个文件齐全。实际采用前重新验证文件树、状态和版本。新文档位于项目根，不受上游子目录规则的写入限制；修改上游 fork 时需重新匹配其规则与技能。

## 关键源码证据入口

| 事实 | 固定来源 |
| --- | --- |
| CPR代理协议、出站选择与后台覆盖 | [代理值对象](https://github.com/zyycn/codex-proxy-rs/blob/58cbb607505dda4a1cee6e01b9f76e267ad3c261/backend/crates/gateway-core/src/account/proxy.rs)、[gateway-egress](https://github.com/zyycn/codex-proxy-rs/blob/58cbb607505dda4a1cee6e01b9f76e267ad3c261/backend/crates/gateway-egress/src/lib.rs)、[出口合同](https://github.com/zyycn/codex-proxy-rs/blob/58cbb607505dda4a1cee6e01b9f76e267ad3c261/docs/architecture.md#L1051-L1083) |
| 禁用代理会从账号投影移除 | [Store投影](https://github.com/zyycn/codex-proxy-rs/blob/58cbb607505dda4a1cee6e01b9f76e267ad3c261/backend/crates/gateway-store/src/postgres/provider_accounts/rows.rs#L423-L431) |
| HTTP连接池以账号及代理URL等为键 | [连接池键](https://github.com/zyycn/codex-proxy-rs/blob/58cbb607505dda4a1cee6e01b9f76e267ad3c261/backend/crates/providers/openai/src/transport/client.rs#L56-L110) |
| CPR管理密钥对应共同ApiKey主体 | [AdminAuth](https://github.com/zyycn/codex-proxy-rs/blob/58cbb607505dda4a1cee6e01b9f76e267ad3c261/backend/crates/gateway-api/src/admin/auth.rs#L83-L107) |
| CPR OAuth start/complete管理接口 | [API合同](https://github.com/zyycn/codex-proxy-rs/blob/58cbb607505dda4a1cee6e01b9f76e267ad3c261/docs/api.md#L1220-L1253) |
| New API转发路径与固定渠道鉴权 | [OpenAI adaptor](https://github.com/QuantumNous/new-api/blob/8542c9d6490b4330fe18160930f81ae7428bc777/relay/channel/openai/adaptor.go#L178-L240) |
| Cockpit订阅URL下载与预算 | [catalog](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/src/modules/codex_proxy_catalog.rs#L956-L1199) |
| 名称型节点ID与原生provider刷新 | [parser id](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/src/modules/codex_proxy_subscription_parser.rs#L59-L71)、[provider bootstrap](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/src/modules/codex_proxy_native_profile.rs#L529-L543) |
| 稳定入口与引擎租约 | [desktop router](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/src/modules/codex_proxy_desktop_router.rs#L1-L68)、[runtime](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/src-tauri/src/modules/codex_proxy_runtime.rs#L612-L807) |

完整的接口、格式与复用清单保存在专题报告，避免在本页重复复制。

## 许可与来源保留

- CPR：[Apache-2.0](https://github.com/zyycn/codex-proxy-rs/blob/58cbb607505dda4a1cee6e01b9f76e267ad3c261/LICENSE)，保留归属、许可和适用的修改声明。
- New API：[AGPLv3](https://github.com/QuantumNous/new-api/blob/8542c9d6490b4330fe18160930f81ae7428bc777/LICENSE)及[NOTICE](https://github.com/QuantumNous/new-api/blob/8542c9d6490b4330fe18160930f81ae7428bc777/NOTICE)。修改版向朋友提供网络交互，同样要按适用条款提供对应源码；保留 UI 归属和项目链接。
- Cockpit：[CC BY-NC-SA 4.0声明](https://github.com/jlcodes99/cockpit-tools/blob/c004bf71be250cc2536fc91ef90638837f0013a0/README.md#L515-L521)。按用户明确的非商业朋友团队场景研究复用；分享改编代码时保留来源、修改与同许可义务，不将抽离代码重新标成仅 Apache-2.0。
- Mihomo：[GPLv3](https://github.com/MetaCubeX/mihomo/blob/v1.19.33/LICENSE)。分发引擎／镜像时处理源码获取和相应分发资料；使用独立进程不自动免除许可义务。

混合仓库或独立 crate 的边界不是自动的许可兼容结论。X02 应形成逐文件复用和分发说明；保留各自许可，必要时以独立组件方式交付。未来用途改变时重新核实，不沿用本轮非商业前提。

本轮不重复商业售卖论证；开源源码许可仍不等同于上游账号服务授权。实际联调使用有权使用的账号，遵守适用服务合同，不把独立 IP 描述为不受限承诺。

## 规划中的源码与部署布局（不表示服务已交付）

```text
项目根/
  README.md / AGENTS.md
  docs/                     持久规划、状态、决策和证据
  research/                 固定版本调研；checkouts不当作产品
  services/
    new-api/                固定版本fork；React + Go适配
    codex-proxy-rs/          固定版本fork；新增无界面出口模块
  deploy/                   固定镜像/二进制版本与内部网络
  tests/integration/        跨服务验证与脱敏样本
  .runtime/                 被忽略的临时运行材料；不存长期事实
```

采用用户指定的 [xinchen6666/xinapi](https://github.com/xinchen6666/xinapi) 单一源码仓库。X01 建立文档基线，X02 按固定 SHA 引入可追踪的实际产品源码和必要锁文件，保留各组件路径、归属及许可；不使用意外 gitlink、不可得的个人 fork 或研究 `.git` 代替源码。上面的服务和部署目录都是未来交付。
