# 验收矩阵与证据

更新：2026-11-22 UTC。前半部分是待执行验收，后半部分才是实际记录。状态权威为 [STATUS](STATUS.md)。本轮没有执行业务构建、代理内核、真实账号、支付或端到端测试。

## 待执行验收矩阵

| 场景 | 对应任务 | 可观察通过条件 |
| --- | --- | --- |
| V01 格式样本 | X06 | 自造Clash YAML/JSON、Base64文档、URI列表、Base64 URI被确定性解析；不支持协议逐项说明 |
| V02 拉取边界 | X06 | 慢响应、超限正文/解压、跳转、401/403和无效UTF-8受限；敏感认证不跨域转发；无越权内网访问 |
| V03 provider与配置 | X06 | 受控拉取并物化；运行内核不再下载远端provider或rules；文件型/递归/不支持资源明确拒绝 |
| V04 节点身份 | X07、X09 | 排序和改名不改变绑定；同名连接参数/秘密变更生成候选；重复指纹不随便选首项 |
| V05 刷新事务 | X07、X09 | 失败保留旧成功版本；成功缺失节点停止新请求；并发旧结果不覆盖新批准；重启后事实一致 |
| V06 引擎资源 | X08 | 进程按账号复用、有界启动、配置核验、正常取消/异常退出可回收；无无限重启和遗留监听 |
| V07 稳定入口 | X08、X09 | 重启端口与CPR proxy一致；冲突时停发或事务同步；跨账号认证与连接不混用 |
| V08 必须出口 | X10 | 禁用/删除代理、引擎故障、损坏绑定、读库失败均不直连；环境变量/NO_PROXY不绕过显式策略 |
| V09 全链路出口 | X10 | 推理HTTP/SSE、WS、OAuth交换、refresh、额度、目录和资料分别有账号出口证据 |
| V10 连接代次 | X09、X10 | 固定URL下换节点不复用旧HTTP2/WS进行新请求；旧在途请求按明确排空规则结束 |
| V11 管理权限 | X05、X12 | 未登录/普通成员直接请求接口失败；管理员有效；伪造角色/身份头失败；秘密不进浏览器 |
| V12 OAuth flow | X11 | 管理员A的flow不能被B完成；过期/重放/换账号或代理版本均受控；CPR提交成功但响应丢失、BFF重启后按原operation查询确认回执恢复；缺失/过期回执保持unknown，禁止从账号列表推断成功或重新兑换code |
| V13 请求白名单 | X05、X12 | 任意URL/path/method、敏感导出或系统操作不能通过BFF泛化代理执行；审计关联本地actor |
| V14 管理界面 | X12 | 登录、订阅预览、节点差异、授权、绑定和错误提示可用；记录实际截图及浅/深/窄屏状态 |
| V15 成员隔离 | X13 | 同名session/thread/cache key不串状态；不能跨成员续接response；改组/封禁/撤销有明确结果 |
| V16 协议链路 | X14、X15 | 两账号两出口两成员完成Responses非流、SSE、WS、工具调用与多轮；保留必要协议字段 |
| V17 故障计量 | X14、X15 | 取消、断流、401/429/5xx和重试有明确用量结果；无无依据重复生成；估算与真实usage可区分 |
| V18 公网与浏览器 | X15 | 每账号双栈观测符合预期；漂移可报告/停发；授权浏览器与服务端路径分别测量，不能仅验服务器 |
| V19 恢复 | X17 | 数据库/秘密/配置恢复后绑定、成员权限和版本一致；升级失败可恢复已知版本 |
| V20 交付 | X16、X18 | 启动说明可复现、内部管理端口不公开、制品版本可核验、许可与修改信息随交付提供 |

这些是未来验收项，当前均为 `not_run`，不是“测试失败”。每个测试记录应注明模拟或真实环境、提交版本、观察到的结果与未覆盖范围。

## 实际记录

E001–E006 是 X01 之前的历史证据，仅支持当时的工作与版本；[旧编号对照](PLAN.md#legacy)不构成新的产品验收。

### E001 范围修正与已有工作区

- 对应：D01。
- 日期：2026-11-22 UTC；执行者：lead。
- 类型：user-confirmed / workspace-inspection。
- 操作：读取用户本轮要求；运行pwd；检查根目录条目与研究Git HEAD。
- 结果：当时的项目根目录仅有 research、无根 Git；New API/Cockpit研究HEAD分别为8542c9d6/c004bf71，CPR为固定提交归档。
- 限制：Git HEAD不证明每个研究文件完整；未构建或运行上游程序。

### E002 订阅与固定出口源码核查

- 对应：D02。
- 日期：2026-11-22 UTC；执行者：subscription-plan，集成复核lead。
- 类型：source-inspection。
- 对象：Cockpit c004bf71、CPR 58cbb607，见[专题报告](../research/feasibility-subscription.md)。
- 结果：核实格式分流、下载限制、名称节点ID、原生provider自主刷新、稳定入口和引擎生命周期；识别CPR连接池在稳定URL换节点时的代次缺口。
- 限制：静态源码证据；未执行解析器、Mihomo或网络测试。补充引擎安装源码的单项网络请求超时未用于肯定结论。

### E003 后台集成源码核查

- 对应：D03。
- 日期：2026-11-22 UTC；执行者：admin-integration-plan，集成复核lead。
- 类型：source-inspection。
- 对象：New API 8542c9d6、CPR 58cbb607，见[专题报告](../research/feasibility-admin-integration.md)。
- 结果：核实React/Go与Vue/Rust边界、管理员中间件、管理API Key共享actor及本地OAuth owner需求。
- 限制：未执行BFF、页面、授权或用户越权测试。

### E004 项目记忆一手资料

- 对应：D04。
- 日期：2026-11-22 UTC；执行者：project-memory-plan，集成复核lead。
- 类型：document-review。
- 对象与结果：读到AGENTS规范、OpenAI官方ExecPlan文章源、Anthropic长期harness文章、固定Spec Kit模板；形成轻量分层方案，见[研究报告](../research/project-memory-methods.md)。
- 限制：某些网页403/超时后用可取得的一手替代；未安装任何框架。不代表跨agent交接已经实测通过。

### E005 文档完整性检查

- 对应：D05。
- 日期：2026-11-22 UTC；执行者：lead。
- 类型：document-check。
- 实际操作：Python3 检查根 README/AGENTS、docs下Markdown、research顶层Markdown的本地链接目标、代码围栏成对和遗留占位标记；随后读回关键计划与状态。
- 实际结果：初次检查15份文档、80个本地文件链接，无缺失或结构问题；交接复核修订后再次检查15份文档、82个本地文件链接及4个标题锚点，问题数0。17个持久任务ID无重复、均有计划定义；最终5项规划任务done、12项业务开发任务pending。
- 结论：passed，仅限文件与结构检查。
- 限制：未逐一联网验证所有外部链接、未渲染页面或Mermaid、不是业务测试。后续任何路径/链接变更需重新验证受影响项。

### E006 接手演练与设计一致性复核

- 对应：D05。
- 日期：2026-11-22 UTC；执行者：handoff-review；修订与集成复核：lead。
- 类型：document-review / handoff-exercise。
- 操作：由未参与本轮规划的agent按AGENTS→STATUS→HANDOFF读取本地文件，回答目标、当前完成度、下一任务及验收，不执行程序。
- 结果：能恢复小团队范围、明确12项开发任务未开始、定位D05收尾和后续P0-01，并区分源码阅读与运行验收。
- 发现与处理：专题中AccountEgressBinding与主架构单一绑定权威不一致，已收敛为ManagedEgressRoute；OAuth unknown恢复合同未定清，已新增DEC-013并同步P0-02/P3-01/V12及两个专题。
- 结论：接手可读性通过；两处规划歧义已由lead修订并读回/结构复查。没有将其写成业务修复或运行验证通过。
- 限制：该agent由fork创建，要求以本地文件作答；这不是对所有模型/客户端自动加载文档的保证，也未模拟真实跨工具部署。

### E007 X01：公开仓库文档基线

- 类型：文档完整性与发布前检查；不属于 V01–V20 的业务验收。
- 2026-11-22 UTC：在 `git write-tree` 的候选快照上用 `git archive` 读取 UTF-8 文本，不访问工作区的研究检出。显式白名单为 18 份 Markdown 加 1 份 Git 忽略规则。首次全量检测验证 116 个本地链接、11 个锚点、X01–X18 与 V01–V20 的任务映射；未发现白名单外文件、断链或所测的常见凭据特征。`git diff --cached --check` 通过。外链可用性和所有可能的秘密类型未穷举。
- 发布及后续工作树检查以[X01 工作日志](worklog/X01.md)为准。GitHub 可读不等于可推送；实际远端 ref 与 SHA 才是交付证据。

### E008 X01：同步回执

- 本地基线：`5b310228174ada93e1006c7f97c05e1f575295db`；对象完整性检查成功。
- 2026-11-22 07:53 UTC：实际 HTTPS 推送因未取得本机 GitHub 凭据失败；只读复查目标 `main` 无 ref。此项 `blocked`，不计作 GitHub 已交付；不影响已完成的文档本地校验。详见[X01 工作日志](worklog/X01.md)。

### E009 X01：远端发布已核实

- 2026-11-22 08:18 UTC：用户授权发布后，GitHub API 显示目标仓库当时没有分支；对 `origin` 执行普通 HTTPS push，返回 `[new branch] main -> main`，没有强推或改写既有提交。
- 对 [GitHub main ref](https://api.github.com/repos/xinchen6666/xinapi/git/ref/heads/main) 的独立 HTTP 查询和 `git ls-remote --exit-code origin refs/heads/main` 当时都返回 `bcf6eff7632da4ce00ca48492691fae58131bcc1`，与 `git rev-parse main` 相同。X01 发布验收通过，E008 为已解除的历史阻碍。本文的后续回执提交不改变上述基线证据。
- 范围：文档发布与 Git 历史，不是业务构建、部署或 V01–V20 产品测试；其他任务未开工。

### E010 X02：固定源码导入与内容核验

- 对象：New API `8542c9d6490b4330fe18160930f81ae7428bc777`；CPR `58cbb607505dda4a1cee6e01b9f76e267ad3c261`；X02工作分支 `task/X02-source-baseline`，起点 `24edbd5`。
- New API上游和导入tree均为 `11dbbb8a51210a38b84d9f9358c56c9d0415f5e8`，2,840个文件。CPR上游发布root tree为 `d606f1081ffdeb53dbbd9efd89c6edef9427887e`，其中包含两个开发联调gitlink；本项目明确不留嵌套repo。CPR实际2,074普通文件组成的独立import tree为 `3edc9042af879db4d0d8c5a09c39d2b142f5994c`。两个tree均分别固定、可复算。
- `python3 -B tools/verify_upstream.py`、`--index`及`--git-ref 5b93f04d4805fab67b11ede5f0d605defc9a874a`已通过；逐项校验路径、执行位、大小、Git blob、SHA-256、清单和tree。源暂存文件4,914个，无160000，未导入研究缓存。
- `python3 -B -m unittest discover -s tests -p test_verify_upstream.py -v`最终11项通过。回归覆盖来源pin不可被清单改写、Git意向空文件、owner执行位、可选gitlink、省略路径、嵌套Git和内容篡改。Independent handoff-review指出风险后，已在X02中修正。
- New API原始`VERSION`是空blob；构建生成的`web/dist`不在来源tree。CPR锁文件所列Git依赖可解析，52个本地Cargo path及Docker输入已静态核对。没有安装依赖、运行上游程序或构建镜像。
- 初始源码扫描发现94项凭据形状候选，逐路径检查都属于上游样例、测试、CI或i18n内容；未验证是否能访问，不代表穷举所有秘密。当前包中没有本团队账号、订阅链接或凭据。公开前仍应复核其增量。

### E011 X02：Cockpit、许可及 Mihomo 元数据

- Cockpit固定提交为`c004bf71be250cc2536fc91ef90638837f0013a0`；已保存来源和逐模块抽离清单。基于许可组合尚未明确，本步没有复制任何Cockpit业务实现。
- Mihomo `v1.19.33` 的tag commit固定为`3f27ef39f8c96bbfbc8b0b9bb10f5c9ef60149d8`；记录Linux amd64-compatible、arm64官方压缩资产ID、大小和release SHA-256，许可文件SHA已和Git blob对照。二进制未下载、未解压、未执行、签名未验、压缩外摘要未知；实际采用和分发前需按清单复核GPL对应源码。
- 以上是X02源和元数据验收，不是产品构建、运行、授权、出口或V01–V20验收。

## 未来记录模板

每个run使用唯一证据ID，写：任务ID；checked_at（时区ISO时间）；执行者；证据种类；真实提交/工作区摘要；脱敏环境；实际命令/步骤；预期；实际；`passed/failed/partial/skipped/not_run`；本地持久证据路径；缺口；失效条件。未执行的命令只放计划，不放实际记录。

与真实账号、代理或页面有关的证据不要只保存于聊天或临时工具输出；整理脱敏材料到后续创建的 `docs/evidence/`，由本页索引。不要保存完整订阅URL、Cookie、OAuth回调code/state、节点密码或token转储。
