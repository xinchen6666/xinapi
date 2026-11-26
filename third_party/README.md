# 第三方来源与 X02 导入边界

X02 原样引入 New API 和 CPR 的固定源码，不更改其业务逻辑、品牌或许可证。用户代码的发布仓库为 [xinchen6666/xinapi](https://github.com/xinchen6666/xinapi)。源码能取得不等于已完成构建、部署或账号验收；当前进度看[状态表](../docs/STATUS.md)。

| 组件 | 仓库内内容与许可 | 固定依据 |
| --- | --- | --- |
| New API | [完整源码](../services/new-api/README.md)、[AGPLv3](../services/new-api/LICENSE)、[NOTICE](../services/new-api/NOTICE) | [逐文件清单](new-api.provenance.json)，2,840 文件，根 tree `11dbbb8a51210a38b84d9f9358c56c9d0415f5e8` |
| codex-proxy-rs | [正式宿主源码](../services/codex-proxy-rs/README.md)、[Apache-2.0](../services/codex-proxy-rs/LICENSE) | [逐文件清单](codex-proxy-rs.provenance.json)，2,074文件；发布 tree `d606f1081ffdeb53dbbd9efd89c6edef9427887e` 含两个可选 gitlink。本项目的导入 tree `3edc9042af879db4d0d8c5a09c39d2b142f5994c` 仅含这2,074个普通文件 |
| Cockpit | 本步只提供[抽离采用清单](cockpit-adoption.md)和[原许可声明](LICENSES/cockpit-license-notice.zh.md)，未复制 Rust/Tauri 代码进产品 | 固定提交与具体文件在清单中；CC BY-NC-SA 条件不能因非商业自用就自动视为与 Apache/AGPL 混合兼容 |
| Mihomo | [固定 release 制品资料](mihomo.lock.json)及[GPLv3 文本](LICENSES/mihomo-GPL-3.0.txt)，未纳入压缩包或运行二进制 | SHA256 对应官方 release 的 `.gz` 资产字节，不代表已验证解压后的程序；下载与执行留给构建/引擎任务 |

没有将仓库整体标成 Apache、AGPL 或 CC BY-NC-SA；各组件的许可、修改、署名和对应源码分发条件分别适用。Cockpit 未确认的组合部分先保留设计和接口，抽离实现前再确定可用代码及分发方式。这不改变朋友小团队场景，也不排除采用其设计。

## 可选子模块与网络依赖

CPR 原仓库有 `modules/ui`、`modules/plugins` 两个 gitlink。它们只用于 UI/插件源码联调，不参与默认 frontend build 或正式 Docker `runtime` 构建，本次不在父仓库保留 gitlink 或嵌套 `.git`；原 `.gitmodules` 文件作为上游来源文件保留。不能在本仓库把该文件当作已初始化子仓库的证明。

默认构建仍需要下载正常软件依赖，不是离线全依赖包。CPR 的 UI 包按 pnpm 锁文件中的提交和完整性摘要安装，执行其 `prepack`；三个 Git Cargo 来源也按完整提交固定，见[构建来源锁](cpr-build-dependencies.lock.json)。X02 仅核对 commit 可解析、本地路径、Docker 输入和锁文件存在；没有安装包、执行安装脚本或验证所有 registry 归档。New API 的三个 Go 模块和 Bun/npm 锁文件原样保留。实际构建与对应依赖审查由 X03 进行。

New API 的 `VERSION` 在固定提交中是空 blob；前端 `web/dist` 是构建生成目录，不能在 X02 编造版本或伪造构建输出。CPR 的两个可选模块下的链接和源码联调命令不属于本次可执行构建流程。导入的上游 Docker Compose 仅为原始参考；本产品的五服务根 Compose 在 X04 交付，不能直接把两个示例相加当成部署完成。

## 重复验证

在项目根运行，Python 只读取来源数据和 Git 对象，不运行上游：

```bash
python3 -B tools/verify_upstream.py
python3 -B tools/verify_upstream.py --index
python3 -B -m unittest discover -s tests -p test_verify_upstream.py -v
```

第一条核对当前文件的内容/模式/路径集合、Git blob、SHA256和完整上游 tree；第二条只核对实际将提交的 index，`--git-ref`只核对指定历史对象，两者不以工作树文件替代Git对象。工作树模式拒绝 `services`、组件根以及 provenance 所在目录的符号链接；provenance 必须是普通、不可执行的 JSON 文件，Git 模式为 `100644`。

后续修改产品源码后，基线清单保持不可变。使用**包含当前 provenance schema 的已验证 X02 提交**复核原始引入，例如：

```bash
python3 -B tools/verify_upstream.py --git-ref dc5263120fd97ee92ddc39e947b55b4ee3bc9a82
```

最早的源码导入提交 `5b93f04…` 缺少后来增加的 `import_tree_sha` 字段，不适用于上述 schema；不能把它当作同一验证样例。后续发行版本的新增/改动单独记录，不把修改后的哈希写成原上游。

清单不是秘密清理器。X02 扫描上游测试、示例和配置样本中的凭据形状，检查本项目新增资料无真实秘密；公开上游样例不能直接用于部署。不可在清单更新中偷偷纳入用户 `.env`、账号文件、私钥、原始订阅或数据库。
