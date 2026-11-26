# 上游清单与采用边界

更新：2026-11-22 UTC。下列提交是本轮证据基线，不代表运行中的部署，也不承诺未来 main 仍有相同行为。升级必须重新核对受影响接口与许可。

| 组件 | 固定基线 | 仓库内采用内容 | 采用目的 |
| --- | --- | --- | --- |
| [New API](https://github.com/QuantumNous/new-api) | `8542c9d6490b4330fe18160930f81ae7428bc777` | [完整源码](../services/new-api/README.md)；[来源清单](../third_party/new-api.provenance.json) | 原样引入2,840文件，Go／React及锁文件，尚未构建 |
| [codex-proxy-rs](https://github.com/zyycn/codex-proxy-rs) | `58cbb607505dda4a1cee6e01b9f76e267ad3c261` | [宿主源码](../services/codex-proxy-rs/README.md)；[来源清单](../third_party/codex-proxy-rs.provenance.json) | 原样引入2,074文件；两个可选联调gitlink明确省略，正式构建用锁定依赖 |
| [Cockpit Tools](https://github.com/jlcodes99/cockpit-tools) | `c004bf71be250cc2536fc91ef90638837f0013a0` | [抽离清单](../third_party/cockpit-adoption.md)及许可声明；未复制业务代码 | 区分纯解析、适配及桌面层，混合许可待确认 |
| [Mihomo](https://github.com/MetaCubeX/mihomo) | `v1.19.33` → `3f27ef39f8c96bbfbc8b0b9bb10f5c9ef60149d8` | [平台制品锁](../third_party/mihomo.lock.json)；GPL文本；无二进制 | 记录Linux amd64-compatible／arm64的官方压缩包摘要，不等于已运行验证 |

X02 用上游 Git 对象及固定提交树验证实际导入，不依赖研究工作树中的临时改动。两套业务源码保持原样，许可和品牌保留；根仓库没有嵌套 `.git` 或gitlink。完整口径和复查命令见[第三方来源索引](../third_party/README.md)。来源验证通过仅证明字节、模式和构建输入清单，不表示X03构建或运行通过。

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

混合仓库或独立 crate 的边界不是自动的许可兼容结论。X02 已形成逐文件复用和分发说明；保留各自许可，尚未明确的组合实现暂不复制。未来用途改变时重新核实，不沿用本轮非商业前提。

本轮不重复商业售卖论证；开源源码许可仍不等同于上游账号服务授权。实际联调使用有权使用的账号，遵守适用服务合同，不把独立 IP 描述为不受限承诺。

## 源码与后续部署布局

- `services/new-api/` 和 `services/codex-proxy-rs/`：已导入固定版本的实际源码，构建未执行；源码自带协作规则适用于后续相应模块修改。
- `third_party/`：已加入逐文件来源、可选子模块决定、Git构建依赖、Cockpit抽离和Mihomo制品资料。
- `tools/verify_upstream.py`：离线核对worktree、Git index或历史提交的来源完整性；后续改源码时按原始X02提交验证基线，不回写上游哈希。
- `deploy/`、根 Compose和跨服务业务测试：尚未交付，分别由后续X步骤完成。
- `research/checkouts/` 和 `.runtime/`：可选本地缓存，不随仓库公开。

采用用户指定的 [xinchen6666/xinapi](https://github.com/xinchen6666/xinapi) 单一源码仓库。原有上游包依赖依然按锁文件从公开源取得；此项目不声称是离线依赖镜像。工作分支是否已同步或合入以[状态](STATUS.md)和[X02日志](worklog/X02.md)为准。
