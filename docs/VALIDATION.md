# 验收矩阵与证据

更新：2026-11-25 UTC。前半部分是待执行验收，后半部分才是实际记录。状态权威为 [STATUS](STATUS.md)。本轮没有执行业务构建、代理内核、真实账号、支付或端到端测试。

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
- 对已发布提交 `dc5263120fd97ee92ddc39e947b55b4ee3bc9a82` 的独立本地克隆，`python3 -B tools/verify_upstream.py`、`--index`、`--git-ref HEAD` 均通过；逐项校验路径、执行位、大小、Git blob、SHA-256、清单和tree。4,914个源文件，无160000，克隆不含研究缓存。原 `5b93f04…` 缺少后增的import_tree字段，不能用当前schema声称其同样通过。
- `python3 -B -m unittest discover -s tests -p test_verify_upstream.py -v`最终11项通过。回归覆盖来源pin不可被清单改写、Git意向空文件、owner执行位、可选gitlink、省略路径、嵌套Git和内容篡改。x02-review指出风险后，已在X02中修正。
- New API原始`VERSION`是空blob；构建生成的`web/dist`不在来源tree。CPR锁文件所列Git依赖可解析，52个本地Cargo path及Docker输入已静态核对。没有安装依赖、运行上游程序或构建镜像。
- 初始源码扫描发现94项凭据形状候选，逐路径检查都属于上游样例、测试、CI或i18n内容；未验证是否能访问，不代表穷举所有秘密。当前包中没有本团队账号、订阅链接或凭据。公开前仍应复核其增量。

### E011 X02：Cockpit、许可及 Mihomo 元数据

- Cockpit固定提交为`c004bf71be250cc2536fc91ef90638837f0013a0`；已保存来源和逐模块抽离清单。基于许可组合尚未明确，本步没有复制任何Cockpit业务实现。
- Mihomo `v1.19.33` 的tag commit固定为`3f27ef39f8c96bbfbc8b0b9bb10f5c9ef60149d8`；记录Linux amd64-compatible、arm64官方压缩资产ID、大小和release SHA-256，许可文件SHA已和Git blob对照。二进制未下载、未解压、未执行、签名未验、压缩外摘要未知；实际采用和分发前需按清单复核GPL对应源码。
- 以上是X02源和元数据验收，不是产品构建、运行、授权、出口或V01–V20验收。

### E012 X02：工作分支交付与独立检出

- 源码和校验器已交付到 [工作分支](https://github.com/xinchen6666/xinapi/tree/task/X02-source-baseline)，审阅入口 [PR #1](https://github.com/xinchen6666/xinapi/pull/1)，base `main`，尚未合并。X02完成指来源基线验收及分支交付，不把合并与部署算作已发生。
- GitHub核对过的提交 `dc5263120fd97ee92ddc39e947b55b4ee3bc9a82` 的root tree为 `72a5a3338823625db5ab8484d576cecbd3f2a856`，与本地同内容tree一致；两服务import tree分别为 E010 中的 `11db…` 和 `3edc…`。MCP曾产生与本地不同的commit metadata，内容未丢失。
- 标准Git传输间歇失败后，通过GitHub API取得缺失的已发布commit元数据；仅在完整Git commit对象重构SHA精确匹配远端SHA时写入本地对象库。确认tree一致、父对象完备、工作树干净，保存原分支到 `backup/X02-local-before-sync-27fe3a2a1100`，再对齐本地branch和tracking。未删除本地历史，未强推或改写远端。
- 从与远端一致的 `dc526312…` 使用 `git clone --no-local` 建立独立本地检出：三种来源校验、11项回归及 `git fsck --full --no-dangling` 成功，工作树干净，无研究缓存。这是本地独立克隆，不是那次超时的HTTPS网络clone。
- CI/status为空：未声称CI通过；构建与CI属X03，真实应用与V01–V20均未执行。本次收尾只同步验收和交接，源子树及验证器不变；具体最新分支SHA在最终发布后单独核对，不在每次回执中自引用。

### E013 X02：审查修订

- 用户要求暂停X03，对已交付X02审查并修复。起点为 `9d0708fd075045fe0da58473806f61e0d581f031`，仅涉及自有校验工具、测试和记录；[完整审查记录](worklog/X02-review.md)。
- 复现两项错误通过：服务根/父目录链接到外部源码时worktree校验仍成功；manifest Git mode为120000而内容是有效JSON时index/historical仍成功、实际checkout失败。临时目录与隔离index验证，没有修改真实源代码或主index。
- 已修：真实目录检查和100644普通manifest读取；工作树manifest本身/父路径链接拒绝。新增回归证明正常index/history不依赖被改动的工作树。16项工具测试通过，实际4,914个源文件工作树校验通过。历史11项结果保留在E010，不改写旧记录。
- 纠正历史使用说明：当前schema的复查示例指向已验证的 `dc5263120fd97ee92ddc39e947b55b4ee3bc9a82`，不能误选缺字段的第一次导入提交。
- 独立只读复核确认两项已修，index/historical仍只读取Git对象。实际worktree、暂存区`--index`、`--git-ref dc5263120fd97ee92ddc39e947b55b4ee3bc9a82`三种模式均通过；源目录、原始manifest和制品锁/许可相对审查起点无变化。9个预期修改文件，23份项目文档的165个本地链接通过，`git diff --cached --check`通过。
- 不涉及X03或上游应用构建/运行，不更改`services/`、锁文件或许可文本。PR不合并，不部署。最终提交/PR SHA通过GitHub发布后单独核对，不为自引用重复提交。

### E014 X03：实际镜像构建与持续检查

- 基线：X02审查提交 `072e434a528bb9295d9c6d4ee6000c91b03485ca`，源代码/锁/逐文件manifest没有修改。构建适配在根deploy目录，固定Go/Bun/Rust/Node/pnpm及原基础镜像digest。范围见[X03日志](worklog/X03.md)、[构建说明](BUILD.md)。
- New API：Bun1.4.4、Rsbuild2.2.0前端及Go1.26.10（原PGO）实际编译通过。最终本地标记镜像OCI index `sha256:0013ce97c44f35483e51fb77b8932eb0060eeb3e4ea1d245ab4e097d175dec34`，应用二进制SHA256 `9f884a66ef8ba4e532be6487f65cb36a8473d1509f652b0460397818ed95bd3d`。无网络只读短命版本/帮助和许可证对照通过；原VERSION空blob导致版本输出为空，不伪造release。详见[独立证据](evidence/X03-new-api.json)。
- CPR：固定UI包prepack、TypeScript/Vite生产构建和Rust1.99.1 release --locked通过（47m12s）。最终OCI index `sha256:74061ea616944a0a220158d0adb56510520d8299a525a01b6f8db060d1ad6b75`，非root用户cpr，版本3.54.2；帮助、静态资源、PG18.7备份客户端和原Apache许可SHA对照通过。详见[独立证据](evidence/X03-cpr.json)。没有数据库连接或服务端口。
- 网络边界：原Docker Hub路径超时，不据此认定pin不存在。替代registry的每个manifest原始字节SHA与原pin相同；实际层下载仍按同digest。Go proxy显式切换但保checksums/GOSUMDB；Cargo使用临时builder Git CLI/HTTP1.1保全部原commit。失败和主动取消尝试保留，不伪装通过或清空历史。
- CI：提交 `c70bb0c2bc9b22c837d3d1ec87c65492148021c6` 的[run 45101737824](https://github.com/xinchen6666/xinapi/actions/runs/45101737824)实际changes、quality、两个镜像和汇总均成功；不是仅静态工作流验证。Ruff版本/wheel SHA、actions完整SHA、BuildKit版本/digest固定，权限只contents read，无镜像registry推送。
- 发布前复核补严格receipt：缺少BuildKit metadata、导出digest与加载镜像不符、平台/source/revision错误均失败。47项工具回归通过；真实Docker29的index ID/metadata形态通过。已提交快照 `a1da347925957c6f1a0ec50bd8157bb55457246a` 经构建入口本地New API验证通过；其[新版CI run 45104467732](https://github.com/xinchen6666/xinapi/actions/runs/45104467732)changes、quality、两个镜像和汇总均实际成功；该run对应47项工具回归及严格metadata代码。最终记录提交只更改文档/证据，源代码与构建逻辑不变，复用这个未受影响的完整构建证据。
- 以上只覆盖构建、项目工具检查和短命CLI，不代表Go/Rust全部业务unit、管理页面、数据库、OAuth、Clash、固定出口或V01–V20已经通过。cache重建标注为cache重建，APT安全更新/构建时间可能改变字节，不承诺跨日期逐字相同。详细日志在忽略目录，公開只存脱敏摘要。

### E015 X03：CPR 发行版本修订

- 专项审查R1复现了统一入口的空版本输出；历史手工构建显式传入3.54.2，其版本证据只适用于对应命令。用户已确认继续修订，范围为构建版本传递、实际输出比较与交付说明。
- `cpr_release_version`从所选提交的发行文件读取顶层版本；构建参数追加 `CPR_VERSION`，回执记录 `release_version`。CI以回执中的镜像执行版本命令，再与同一提交的发行值比较。
- 现有47项工具回归通过，测试文件保持原样。临时Git样例验证所选提交各自返回3.54.2与3.54.3，工作树中的99.0.0独立保留；实际构建参数包含3.54.2。
- 新CI命令对旧空版本镜像实际返回exit1，日志为expected=3.54.2、actual为空。独立只读复核确认正常3.54.2输出通过，空值和其他版本输出失败。
- 本机修订镜像实际构建成功，使用与CI相同的版本命令得到 `expected='3.54.2', actual='3.54.2'`，exit0；执行条件为隔离网络、只读文件系统和短命容器。
- [PR版本修订CI 45120537811](https://github.com/xinchen6666/xinapi/actions/runs/45120537811)与[push版本修订CI 45120533870](https://github.com/xinchen6666/xinapi/actions/runs/45120533870)的changes、quality、两个镜像及汇总均成功。独立读取两轮CPR作业日志，均显示 `CPR release version verified: 3.54.2`，版本步骤exit0。
- R1版本传递与CI输出比较已完成复核。旧空版本镜像的exit1及修订镜像的exit0分别记录；最终记录提交只整理文档，版本实现和配方保持已验证状态。X04保持pending。

### E016 X04：五服务本机联调

- [X04 工作日志](worklog/X04.md)记录已启动的五个服务、授权连接到各自库、跨库拒绝、独立 Redis 认证和 AOF、宿主回环绑定、首次管理员及保留卷重建后的登录。
- 公开脚本权限导致的初次失败亦保留在日志中；修正后的空卷成功。此为本机 Linux/amd64 隔离项目的功能结果，真实上游流量属于后续工作。

### E017 X04：专项审核

- 2026-11-23（Asia/Shanghai），lead；并行只读审核为 x04-auth-review 和 x04-compose-review。环境为 Linux/amd64、Docker 29.1.3、Compose 2.40.3、CPR 3.54.2、PG18.7、Redis 8.10.3，复用 X03 本机应用镜像。
- 结果 `partial`：[完整专项记录](worklog/X04.md#2026-11-23-专项审核)保留五服务状态、真实认证、网络边界、空卷、保留卷和异常初始化的脱敏结果。正常空卷与重建后的两层管理员、两库和两个 Redis 认证通过；普通成员访问 New API 管理接口 403，跨应用令牌访问 CPR 管理接口 401。
- 通过项：两应用普通数据库角色、跨库拒绝、Redis 独立口令、AOF 及原数据保持、New API 回环绑定、独立外部网络的 19 项连接拒绝、私有文件访问限制、CPR 依赖故障 503 及恢复 204。
- 已复现 P2：Redis 延迟 45 秒时，`up --wait` 8.37 秒 exit 0，New API 54.58 秒才可用；半成品 PG 的单服务等待也曾提前成功。另有 P2 运维说明缺项：原卷半套初始化受保护并明确拒启，直接重跑角色创建 exit 3，原卷补全步骤待完善。
- 三个审核项目已停止，命名卷与私有配置保留；原演练继续运行。应用源码和测试文件保持现状，本轮记录在本地工作树。异机、公网 IPv6、真实上游和后续管理适配为 `not_run`。配置、脚本、镜像或网络规则变化后重新验证对应结果。

### E018 X04：真实就绪与原卷补全修订

- 用户确认修订两项审核问题；实现和实际结果见 [X04 修订记录](worklog/X04.md#已确认修订与复验2026-11-23)。一次性启动工具分别使用两应用配置完成 PG 普通角色认证、Redis PING 及应用接口验证，总时限覆盖启动与验证。
- Linux/amd64 原隔离环境实测：正常保留卷 10.14 秒成功；Redis 延迟 45 秒时，8 秒时限 exit 1，后续 120 秒时限等待 41.71 秒后成功；半成品 PG 5.07 秒 exit 1，明确提示初始化和补救文档。
- 原半套 PG 卷补全后保留原记录；已有 LOGIN 角色遇错误配置 exit 1，原配置成功，原口令保留；完成态重复执行成功。补全后五服务 3.92 秒成功，两个应用管理员及成员原登录、CPR 原管理 Key、两库和两个 Redis 数据均保持。结果 `passed`。
- 原镜像、网络边界与自造业务数据范围沿用 E017；新生产工具的 Python AST、shell 语法和差异格式已检查。测试文件保持现状。参数、脚本、镜像或依赖合同变化后复验相应场景。

### E019 X05：管理员只读入口

- 2026-11-23 UTC，lead；后端实现复核 x05-backend-review，独立只读复核 x04-delivery。版本、命令范围、失败及结果见 [X05 日志](worklog/X05.md)。最终 Go／React 镜像实际构建，类型检查与 lint 通过，原共享表格回归 7 项通过，Go middleware／router 和 authz 其余原测试通过。
- 真实五服务与自造业务数据：CPR 账号空列表及自造认证代理列表成功；私有 Key 错误／文件权限错误安全返回 503，原文件恢复后立即成功。原管理员和成员凭据保持。
- 内部 CPR 夹具：匿名／模型 Key 为 401，成员／缺权管理员为 403，八次拒绝的管理调用为零；两类响应严格限七／五字段。账号第二页为 3 项；原始邮箱、地址、凭据和错误细节均排除，邮箱派生名称使用稳定 ID。
- 浏览器：加载、分页、错误重试、模拟权限撤销、三种路由拒绝及 390px 移动长 ID 显示成功，管理秘密泄漏与运行错误为零。截图入口见 [页面证据](worklog/X05.md#页面证据)。最终验证接回真实 CPR，临时夹具已回收。
- [Draft PR #4](https://github.com/xinchen6666/xinapi/pull/4)已交付，base 为 X04；远端功能代码、证据及 PR 文案和状态读回通过。结果 `passed`（2026-11-24 UTC 更新）：`TestSetUserPermissionsStoresOnlyOverrides` 的两处完整资源对象期望经用户授权加入 `account_egress:read`，`gofmt`、`go vet` 与 `go test ./service/authz/ ./middleware/ ./router/ -count=1` 三个包全部通过。源码、页面与真实权限结果分别记录，失败与修复过程见 [X05 日志](worklog/X05.md#实际验证)。
- 真实上游账号、外部代理、OAuth 与模型流量为后续任务覆盖。管理接口、权限模型、字段或镜像变化后复验对应结果。

### E020 X06：订阅粘贴与预览

- 2026-11-24 UTC，lead；CPR 侧实现与测试由 cpr-preview 执行，New API 侧由 newapi-preview 执行，运行期矩阵与页面截图由 lead 复核。范围、命令与结果见 [X06 日志](worklog/X06.md)。
- CPR：`cargo fmt`、`cargo clippy --workspace --all-targets --all-features -- -D warnings` 与三个 crate 的 `cargo test`（393 / 680 / 217 项）通过，架构测试 16 项通过；解析覆盖 Clash YAML、Clash JSON、URI 列表与 Base64 文档，`proxy-providers` 物化一层。
- New API：`go build ./...`、`go vet` 与 `go test ./service/authz/ ./middleware/ ./router/` 通过；页面 `bun run typecheck`、`bun run build` 与改动文件 lint 通过。
- 运行期：五服务 compose 内以同源代码替换镜像重建，经 New API 接口调用真实 CPR 预览矩阵，覆盖格式、限额、目标策略、权限与参数校验；响应只含白名单字段。截图见 [X06 日志](worklog/X06.md#实际验证)，结构化结果见 [X06 脱敏证据](evidence/X06.json)。
- 缺口：正式镜像经现有 Dockerfile 从源码构建、真实订阅源与多跳重定向的运行期实测留到 X08、X16 与获授权的验收任务。

<a id="e021-x06v01v03专项审核"></a>
### E021 X06：V01–V03 专项审核

- 2026-11-24（Asia/Shanghai），lead；结果 `failed`。本轮按用户给定反例，使用独立 Compose 项目、自造 HTTP/TLS 源、专用测试 CA、一个停用账号与一个既有代理执行实际 API 和浏览器验证。产品源码保持原状。
- 53 个首轮场景、8 个补充 BFF 场景与 1 个 CPR 直调对照；六组常规格式的名称、协议、计数逐项通过。正文与解压限额、约 10 秒下载超时、三次跳转、一层 provider、32 项 provider 上限及跨域认证头清理通过。
- 复现 6 项实现问题：HTTPS 实际私网地址放行、连接参数缺少校验、Base64 长度启发式拒绝合法输入、YAML 语法失败回退 URI、文件型等 provider 静默跳过、CPR 响应放大超过 BFF 限额。部分接受的显式确认属于待补齐条件。
- 账号保持停用，代理 revision=1、全局 configRevision=6 及进程保持原值；规则/外部 UI/深层 provider 访问次数为零；约 217.6 秒空闲期自动拉取次数为零。错误状态隐藏旧行，重试入口存在。
- 三张专项结果表、代码位置与建议见 [审核报告](worklog/X06-review.md)；完整脱敏结果见 [审核证据](evidence/X06-review.json)。当前结论取代 E020 对 V01–V03 充分覆盖的判断；E020 的原构建与交付事实保持归档。
- 实测覆盖隔离网络中的 HTTPS 私网、域名解析私网及跳转/provider 路径。动态 DNS 重绑定、IPv6、真实公网订阅与后续 Mihomo 发布流程列为覆盖边界。

### E022 X06：审核修复复验

- 2026-11-24（Asia/Shanghai），lead 集成，x06-fix-download 与 x06-fix-ui 协作；本地结果 `passed`。修复 R1–R6 及部分接受确认，详情与对应证据见 [修复报告](worklog/X06-fix.md)和[脱敏结果](evidence/X06-fix.json)。
- 下载器逐跳审核全部地址并固定连接集合，复用既有 NetworkPolicy。HTTPS 私网、环回、IPv6 映射地址及混合 DNS 均拒绝；模拟公网 TLS 与单次 DNS 地址固定实测成功；认证头、Cookie 和管理 Key 命中次数为零。
- 72 个真实接口场景全过：地址/端口/UUID/载荷有效性、短/大 Base64、YAML 语法与 flow 映射、provider 诊断、候选与响应预算。4096 个长名节点成功，原 18000 节点样本转为明确的 422 容量错误。
- 真实浏览器：选择有效节点、跳过项与诊断确认、Mihomo 元数据选择、错误/新预览/重新选行清除确认、移动禁选均通过；确认操作请求增量为零，运行错误零；业务账号、代理和配置版本保持原值。
- 最终 Rust 完整回归 399 / 680 / 218 项，架构 16 项；格式和严格 Clippy 通过。Go build/vet、authz/middleware/router 全过；前端类型检查 1513 文件、2 项目，改动文件 lint/format、生产构建及现有表格 7 项回归通过。
- GitHub [整轮 CI](https://github.com/xinchen6666/xinapi/actions/runs/45440635026) 已完成且 success，包含质量、两服务源码镜像与汇总。文档链接及锚点已修正，远端检查通过。
- 本轮功能复验取代 E021 对当前实现的失败判定，历史审核事实保留。修复源码、记录与截图已普通推送至原 Draft PR #5，常规合并保留两边历史；发行镜像全新安装与真实上游验收保持各自范围。

<a id="e023-x07来源与批准版本"></a>
### E023 X07：来源与批准版本

- 2026-11-25（Asia/Shanghai），lead；范围为 X07 首个可审阅增量，结果 `passed_scoped`。15 个主要业务流程和 6 个补充场景实际通过，见 [工作日志](worklog/X07.md)与 [结构化证据](evidence/X07.json)。
- 创建、编辑、刷新、显式批准、改名和排序保持身份、连接变化生成新版本、部分结果保留旧节点、完整缺失确认、旧刷新 409 均通过。默认字段等价性验证通过，连续 65 次相同批准复用原批准版本。
- 来源文档以 AES-256-GCM 保存在 PostgreSQL，部署密钥独立持久化；真实数据库的来源链接与节点地址标记检索结果均为零，密钥模式0600、长度32字节。缺失或其他部署密钥的读取/创建均503，原来源数量保持。
- 权限实测：管理员可读、成员拒绝、超级管理员会话可写；root所属只读PAT的4项来源写入均403。完整响应只含公开DTO，私密链接输入提交后清空。
- 空库和X06既有库追加迁移成功，旧代理保留；升级重复启动和主环境重启后，来源、节点与版本保持原值。正式测试文件保持原状，现有迁移清单的手工摘要操作按当前约定省略。
- 格式、Rust改动库Clippy和网关构建通过；现有订阅25项、API7项、架构16项通过。Go build/vet及authz/router/middleware通过；前端1520文件/2项目类型检查、改动lint、生产构建通过，共享弹窗3项、表格7项通过。
- 桌面和390px移动页面的创建、改名建议、批准、失败保留、私密链接隐藏及管理员只读通过，运行错误零。截图：[差异](evidence/X07-source-review.png)、[批准](evidence/X07-source-approval.png)、[移动](evidence/X07-source-mobile.png)。
- 首个增量已交付 [Draft PR #6](https://github.com/xinchen6666/xinapi/pull/6)，base 为 X06；完整源码与四张截图已送达并读回。CI质量与New API镜像通过，CPR构建继续跟随[运行页](https://github.com/xinchen6666/xinapi/actions/runs/45552189412)。
- 较早整仓测试保留56项前端失败/超时及controller整包超时记录；限定业务场景通过与整仓结果分别记账。真实账号绑定、稳定出口及Mihomo执行属于后续任务，当前available是已批准来源节点状态。

<a id="e024-x07专项复审修订"></a>
### E024 X07：专项复审修订

- 2026-11-25（Asia/Shanghai），lead；范围为 X07 专项复审提出的三项修订，结果 `passed_scoped`；复审事实与修订对照见[工作日志](worklog/X07.md#专项复审与修订)。
- R1 连接事实：socks5h 在 URI 解析写入 dns-mode=remote 并纳入连接键；同一来源切换解析位置返回 connection_changed，管理员显式选择旧身份后 UUID 保持、连接版本追加。
- R2 凭据类型：username/password 要求字符串、tls 要求布尔值；畸形标量在解析阶段拒绝，候选 parse_valid=false、review.complete=false，批准路径保持拒绝。
- R3 编辑器版本：编辑器按打开时 revision 提交；服务端版本已推进时两次保存均返回 409 subscription_source_conflict，并发编辑的名称保留，刷新与批准继续可用。
- 复验环境：真实应用、自造来源、隔离数据卷；审计环境把 CRITICAL_RATE_LIMIT 调至 200 次/20 分钟以支持重复运行，产品默认 20 次/20 分钟。
- 通过项：六点专项 17 项、权限与存储 6 项、后端语义 2 项、cpr 重启后 61 个来源逐项一致、浏览器主流程与编辑冲突，页面运行错误零。
- 编译：Rust fmt/clippy、现有订阅 25 项、API 7 项与网关构建通过；New API go vet/go build、前端 1520 文件/2 项目类型检查与生产构建通过。
- 截图：[编辑冲突](evidence/X07-review-editor-conflict.png)、[来源管理](evidence/X07-review-source-management.png)。修订推送至 [Draft PR #6](https://github.com/xinchen6666/xinapi/pull/6)，base 为 X06。

<a id="e025-x08受管引擎入口"></a>
### E025 X08：受管引擎入口

- 2026-11-25（Asia/Shanghai），lead；范围为受管 Mihomo 引擎入口的监督、观测与稳定端口，结果 `passed_scoped`；实现与边界见 [X08 工作日志](worklog/X08.md)。
- 启动：真实 Mihomo `v1.19.33` 在 CPR 容器内以唯一批准节点启动，冷启动 52ms（首个实例 207ms），RSS 约 42MB，CPU 0.06s；入口只监听 `127.0.0.1`，同网络其它容器连接返回 errno 111。
- 出口：经入口请求目标站点返回 `engine-ok`，自造节点代理计数 2→3，确认流量经批准节点转发。
- 生命周期：同一入口并发启动得到唯一 PID；`kill -9` 后状态 `failed` 并记录退出信号，端口释放，重启复用原端口；停止后入口与控制面端口均释放。
- 冲突与容量：入口端口被占用时启动返回 409 `engine_port_conflict` 且端口保持；第三个实例返回 429 `engine_capacity_reached`。
- 重启：CPR 重启后三个入口均为 `stopped`，端口 24100/24102/24104 保持，容器内残留监听清零；再次启动复用原端口。
- 权限：入口目录与 `config.yaml` 实际为 `600`；控制面密钥随机生成，只存在于本机配置。
- 编译：Rust 格式检查、改动 crate 的 Clippy `-D warnings`、网关构建通过；架构 16 项、订阅 25 项、API 7 项通过。
- 边界：账号绑定与换路属 X09，严格出口属 X10，管理页面属 X12；Mihomo 二进制留在镜像外，验证使用本机下载制品。

<a id="e026-x08生命周期专项审核"></a>
### E026 X08：生命周期专项审核

- 2026-11-25（Asia/Shanghai），lead；审查对象 task/X08-engine-supervision 的 d06398d；结论 requires_changes。范围为 V06/V07，真实 CPR/Mihomo、独立数据库与自造节点；[审核报告](worklog/X08-review.md)和[结构化结果](evidence/X08-review.json)保存完整步骤、发现及资源观测。
- 官方发布 `.gz` 摘要与登记一致，解压 ELF 与实际执行文件一致。失配 ELF 副本保持相同版本文本，启动返回200/running，实际执行摘要为失配值。
- 配置及隔离：回环连接限制和 controller 认证通过；数据入口接受空／错／交叉认证，账号 owner 参数缺少。tls:true 节点生成明文配置，受控明文代理收到有效认证字段。当前目录0700、文件0600，E025中的“目录600”以本次实测更正。
- 生命周期：正常停止约5.075秒后回收，外部 kill 程序缺失；真实 SIGSTOP 使启动约5.064秒超时并回收，其他入口管理操作耗时约10.05秒。数据端口占用409且保持端口；controller端口占用仍running。客户端断开后约0.389秒回收；双引擎正常容器重启的旧PID及监听均清零、端口保留。
- 稳定身份：登记损坏后空列表，B重新得到原A端口；登记写失败后的重试返回running但持久记录缺失。来源UUID只改大小写也会产生另一PID与端口。
- 资源：单实例空闲40.94MiB；双实例空闲合计81.60MiB；10次自造请求后合计85.92MiB，累计CPU增加0.04s。审核上限2，第三实例429；实际Memory/NanoCpus均0，默认4实例和持续负载留待验证。
- 审核栈已停止，原栈保持；源码与正式测试文件保持原状。X08状态调整为in_progress，下一动作按报告修订并复验；上一轮编译与CI记录只支持其原覆盖范围。

<a id="e027-x08审核修复复验"></a>
### E027 X08：审核修复复验

- 2026-11-26（Asia/Shanghai），lead；范围R1–R9与SOCKS DNS，结果passed_scoped。完整[修复记录](worklog/X08-fix.md)与[脱敏结果](evidence/X08-fix.json)保存61项运行场景，原问题对应E026。
- 两个通过管理接口导入的自造账号共用同一批准节点时仍独立PID／端口／凭据；正确认证200，空407，错误及交叉403；controller错误及交叉密钥401。TLS代理200，明文陷阱502。
- 真实ELF追加字节与同长度篡改均503且PID为空；固定入口和controller占用均409且端口保持。SOCKS remote传递原域名ATYP3，本地／缺省DNS422且保持原登记。
- 登记损坏、缺失与写失败均503；恢复文件后端口保持，写失败重试持久化成功并跨重启保留。v1记录保留端口占位，公开DTO保持凭据零暴露。
- 正常停止0.049s，SIGKILL后主动回收；初始化超时5.045s时另一个入口列表／停止均0.034s；客户端断开约0.217s回收；Host双引擎关闭0.759s，端口与凭据跨重启保持。
- 实测双实例RSS约43.76／43.52MiB，累计CPU各0.07s，冷启动含制品校验503／519ms。第三实例429；容器1GiB、2CPU、256PID，单流日志注入3MiB后文件仍1MiB。
- 格式、改动crate全target严格Clippy、订阅25项、API7项、架构16项及网关构建通过；正式测试文件保持原状。4实例、长期负载及真实公网业务留待验证。
- 隔离复验服务已停止，原栈保持。[PR CI 45667344429](https://github.com/xinchen6666/xinapi/actions/runs/45667344429)质量检查、CPR镜像与汇总通过，New API镜像按路径跳过。修复已送达原Draft PR，交付状态与后续动作见状态表和交接。

<a id="e028-x09账号绑定及换路"></a>
### E028 X09：账号绑定及换路

- 2026-11-26（Asia/Shanghai），lead；范围X09，结果passed_scoped。真实CPR/Mihomo、PG/Redis、自造导入账号与独立节点A/B；[工作日志](worklog/X09.md)、[结构化证据](evidence/X09.json)保存运行范围。
- 账号outboundProxyId唯一持有实际代理关系，候选prepare与ready期间原出口保持。断开候选502／failed；批准后专用代理route_generation推进，credential_revision保持。
- 固定endpoint及凭据下，切换前HTTP2和WS经A，切换后新连接经B。旧HTTP2流与WS均关闭；旧HTTP2会话再次请求得到ERR_HTTP2_INVALID_SESSION。
- 旧generation批准409、同operation重复批准保持幂等；客户端丢弃批准响应后GET确认applied；applying中断与CPR重启后显式reconcile完成发布。
- 来源在applying阶段变动导致发布冲突，对账恢复原节点并清除挂起状态；来源后来批准新版本，旧路由重启仍从固定历史连接恢复。
- 通用proxy写、跨账号借用、受管账号删除和携带入口凭据的导出受到保护；初次绑定的原普通proxy在applying期间编辑／删除及账号改绑均409。ready候选abort后可终结并释放状态。
- 格式、相关crate严格Clippy、订阅25、API7、OpenAI代理回归7、架构16与网关构建通过；正式测试文件保持原状。追加0047迁移在隔离既有库执行成功，既有迁移保持原字节。
- 内网双出口观测只证明受控链路，真实公网IP、真实推理协议和用量留待后续任务。候选验证临时需要一个空闲引擎槽，发布策略为显式终止在途流。
- 41项运行场景通过，包括终结候选账号删除后的历史回执。验证服务已停止，原栈保持；[PR CI 45690896769](https://github.com/xinchen6666/xinapi/actions/runs/45690896769)质量检查、CPR镜像与汇总成功，交付及下一动作以状态表和交接为准。

### E029 X10：严格出口与故障停发

- 2026-11-26（Asia/Shanghai），lead；结果passed_scoped。[工作日志](worklog/X10.md)与[结构化证据](evidence/X10.json)记录真实CPR／Mihomo／PG／Redis、自造双节点、默认出口记录器与模拟HTTP／TLS／SSE／WS上游
- 16类OpenAI／xAI实际发送共331次观测，含165次故障／收紧调用；所有故障调用及时拒绝，上游请求增量零、默认出口增量零。正常与恢复阶段重新送达，10项投影／清理检查及12项实际网关场景通过
- 覆盖代理禁用、解绑、generation及URL偏离、账号停用、applying、来源停用、引擎停止与真实异常退出、数据库关系读取失败，另验证旧proxy=None客户端、NO_PROXY=*和存量ResponseControl撤权。删除保护与授权归属均409，DB引用约束也拒绝直接删除受管代理
- 默认出口以独立请求校准，确实记录到流量；故障期间保持零增量。节点A/B与上游均为隔离自造服务，实际公网出口与真实第三方认证保持待验证
- 格式、相关crate全target严格Clippy、网关构建通过；已有回归pending凭据9、OpenAI代理7、token25、xAI refresh26、WS控制10、背压清理3、架构16、bootstrap28通过。正式测试文件保持原状
- xAI交换与JWKS验证夹具刻意拒绝验证，部分目录／订阅响应只用于发送观测；business_completed与发送通过分别记账。背压队列撤权位置经过源码复核与既有回归，真实撤权观测使用空闲原连接控制消息
- 0048新增sticky要求并回填既有受管账号；故障后的字段、来源与数据库关系名均已恢复，验证引擎与专用服务已停止。公共发行／DNS／OIDC预热沿用部署语义；X11承接初次授权预留归属。交付与下一动作以状态表和交接为准

### E030 X11：管理员OAuth与回执恢复

- 2026-11-26（Asia/Shanghai），lead；结果passed_scoped_with_known_regression_difference。[日志](worklog/X11.md)与[结构化结果](evidence/X11.json)记录真实New API／CPR／Mihomo及自造OAuth上游与丢响应中继
- 38项运行检查通过：双管理员与多登录会话隔离、角色撤销、单次并发兑换、CPR提交后断响应恢复、BFF／CPR重启读回、缺失／过期回执unknown、代理版本变化拒绝交换、错误state、过期／未知／重复取消及入口回收；安全会话跨源POST403
- SQLite／MySQL8.4.13／PostgreSQL18.7各fresh与upgrade共6条真实路径，每条28项，共168项通过，含两次迁移、既有数据、owner／权限／期限、并发领取与重连接保留
- 浏览器完成来源／节点选择、授权单次确认、回调清空及完成回执、取消确认与终态。截图：[表单](evidence/X11-form.png)、[待完成](evidence/X11-pending.png)、[完成](evidence/X11-completed.png)、[深色](evidence/X11-completed-dark.png)、[窄屏](evidence/X11-mobile.png)、[取消](evidence/X11-cancelled.png)
- Rust格式、严格Clippy、构建及既有凭据31／OAuth8／架构16项通过；Go构建及相关middleware回归通过；Web1530文件类型检查、限定lint与生产构建通过。正式测试文件修改数零
- 已有Go回归TestSetUserPermissionsStoresOnlyOverrides固定期待旧权限映射，当前新增authorize:true导致1项断言失败，作为明确回归差异交付。xAI真实发现与交换后刻意拒绝自造签名，unknown保持单次；真实公网授权、同出口浏览器与长期负载仍待验证
- 0049在隔离既有库执行成功，凭据／代理／route／X09确认历史／完成回执原子提交。GET保持纯读，授权10分钟与确认回执24小时分别判断。交付及资源收尾以交接与状态表为准

### E031 X02–X11：主干收拢

- Go 1.26.10 复跑指定账号出口权限映射用例通过；登录会话来源、PAT 路由声明与权限目录既有回归通过。唯一正式测试差异为获准的两处字段期望。
- 项目链接、凭据特征检查、15 项既有工具回归及 Compose 配置解析的执行记录见[收拢日志](worklog/X11-consolidation.md)。产品运行范围沿用对应历史证据，X12 业务接口与正式安装留待后续阶段。

### E032 X12：管理员工作台

- [X12 工作日志](worklog/X12.md)记录实际 Go、Rust 与前端构建、指定权限用例、七语、隔离服务中的账号管理和工作台交互。
- 隔离环境使用真实 New API、CPR、PostgreSQL、Redis 及 Mihomo，自造订阅和上游仅供业务验证。验证了满槽释放、过期与活跃预留区分、慢上游与断连时回收、固定版本恢复、并发组更新和双边审计。

### E033 X13：成员授权与内部身份

- [X13 工作日志](worklog/X13.md)记录 Go／Rust／Web 实际构建、既有权限与续接回归；正式测试源码保持原样。托管成员续接记录缺失和存储失联的外置一次性场景调用真实 Core，均返回拒绝。
- SQLite、PostgreSQL 18.7 和 MySQL 8.4.13 的新库、X12 升级与重复迁移均通过，原成员数据保留。真实 New API 与自造 CPR 联调通过两成员独立凭据、授组与并发改组、响应丢失回读、撤权及 WS 下一轮停发。
- 实际浏览器完成登录、用户菜单、确认授组和撤权、390px 窄屏交互；截图见日志。公网模型、完整工具调用及生产部署由后续阶段验证。

### E034 X14：模型数据面与成员用量

- [X14 日志](worklog/X14.md)记录锁定 CPR、New API 与前端构建及既有用量回归；正式测试源码保持原样。
- 隔离真实 New API／CPR／Mihomo 与自造 OpenAI 上游联通同步 Responses、SSE、原生 WS 多轮与函数回填。缺用量、显式零、部分输出、失联、流超时及客户端取消均有实际结果，原生续接按成员隔离。
- 普通成员日志中的三种数量来源、实际详情与窄屏卡片经浏览器操作验证。累计用量与消费记录相符；公网模型及长期负载由 X15 接续。

### E035 X15：完整业务路径与双栈私网出口

- [X15 工作日志](worklog/X15.md)及[结构化结果](evidence/X15-integration.json)记录新隔离栈连续 23 条安全结果，从来源审批和受控 OAuth 单次交换走到成员模型调用、工具回填、换路、故障恢复及撤权；正式测试源码保持原样。
- 目标端实际 socket 观测到所选节点的私网 IPv4 和 IPv6，阻断节点及撤权后目标生成增量为零。成员累计用量与消费记录相符，20 条管理动作核对了持久审计关联。
- 补齐已发布路由恢复的当前请求审计，原配置 revision 与 generation 保持；4 项既有审计回归、实际恢复和浏览器页面验证通过。官方浏览器登录、第三方公网账号／节点、公开出口和长期负载保留待验。

### E036 X16：新克隆单机 HTTPS 首次使用

- [X16 日志](worklog/X16.md)及[结构化结果](evidence/X16-installation.json)记录 Linux amd64 新克隆、明确本地应用镜像、空 PostgreSQL 卷、两套 Redis、只读 Mihomo 与单机 nginx 的实际运行。凭据和配置保持私有，正式测试源码保持原样。
- Chromium 经受信证书完成首次设置和登录；首次 CPR Key 显式导入后管理页面可用。通过 HTTPS 完成来源审批、批准节点、受控 OAuth 单次交换、成员授组、实际渠道 ID 激活和模型 JSON／SSE。WSS 经 nginx 完成同连接两轮续接，撤权后的下一轮停发；成员用量合计相符。
- 宿主应用端口仅 New API 回环，验证浏览器与模型客户端均使用 HTTPS。官方浏览器授权、第三方公网账号及节点、公开发行和生产部署保留待验。

### E037 X18：候选交接与首次调用

- [X18 日志](worklog/X18.md)和[结构化结果](evidence/X18-handoff.json)记录 Linux amd64 应用候选包实际导出与重新载入，并在独立空卷以新私有配置启动。Mihomo 只读，Docker 实际发布仅 New API 回环端口。
- 经可信测试 TLS 完成首次设置、显式导入管理 Key、受控 OAuth 单次交换、成员授组及实际渠道激活。HTTPS JSON／SSE／WSS 和[接手示例](RELEASE.md#2-从导入走到成员首个请求)通过，续接归属、撤权停发及用量记录相符。正式测试源码保持原样。
- 候选源包、镜像包和按组件保留的许可说明用于本机交接；公开发布、生产部署、正式公网授权与节点、其他架构及 X17 继续保留待验。

## 未来记录模板

每个run使用唯一证据ID，写：任务ID；checked_at（时区ISO时间）；执行者；证据种类；真实提交/工作区摘要；脱敏环境；实际命令/步骤；预期；实际；`passed/failed/partial/skipped/not_run`；本地持久证据路径；缺口；失效条件。未执行的命令只放计划，不放实际记录。

与真实账号、代理或页面有关的证据不要只保存于聊天或临时工具输出；整理脱敏材料到后续创建的 `docs/evidence/`，由本页索引。不要保存完整订阅URL、Cookie、OAuth回调code/state、节点密码或token转储。
