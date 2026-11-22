# X02 审查与修订记录

审查起点：`9d0708fd075045fe0da58473806f61e0d581f031`；目标：[PR #1](https://github.com/xinchen6666/xinapi/pull/1)，继续使用 `task/X02-source-baseline`。用户明确要求只审核X02、修复并重新推送，暂停X03。日期：2026-11-22 UTC。

## 审查范围与结论

检查X02新增的离线来源校验器、测试、来源及制品锁、许可边界和交接记录；两个子代理分别只读审查校验逻辑与范围/证据，lead复现实质问题并集成修复。没有审计全部上游业务逻辑，不构建/运行New API、CPR、Mihomo、Docker或数据库。

X02总体符合[源码基线需求](../PLAN.md#x02)。New API 2,840文件和CPR 2,074文件按当前提交逐Git blob/模式/哈希与来源清单匹配，必要锁文件和本地构建输入存在。没有发现本次导入缺件或错误源版本。固定源码存在仍不代表Clash、OAuth、出口或统一后台集成已完成。

## 已修复的问题

| 编号／影响 | 原行为与复现 | 修复与验证 |
| --- | --- | --- |
| A1 中：工作树会错误接受外部源码缓存 | 临时项目把`services`或`services/new-api`改成指向外部目录的符号链接，文件字节正确时原默认校验仍成功。以`base.resolve()`为边界会把被重定向目录本身当成可信根 | 从项目根逐段要求服务目录为真实目录；新增两种目录链接场景，修改前均失败（未抛出预期错误），修改后均拒绝 |
| A2 中：不可正常检出的来源清单仍能通过 | 在隔离Git index里，把原JSON manifest的blob仅改mode为120000。原`main`丢弃`git.read`的mode，`--index`仍exit0；子代理也复现历史tree模式通过，而checkout尝试创建长JSON目标的symlink失败 | 新增manifest专用读取，Git对象模式必须100644；工作树manifest与其父目录不可为链接，manifest必须普通不可执行的JSON对象。测试覆盖正常文件、symlink、执行文件、index/history和工作树父路径 |
| A3 文档：基线提交示例易选错 | “X02原始导入提交”可能被理解为`5b93f04…`，但该早期版本没有当前schema的`import_tree_sha` | [来源索引](../../third_party/README.md)改用已验证、包含当前schema的`dc5263120fd97ee92ddc39e947b55b4ee3bc9a82`，明确旧提交不适用 |

A1、A2是校验工具的错误通过边界，不代表本次已导入源码发生上述替换。实际服务根目录、manifest模式和来源内容都正常。修复未修改任何`services/`或逐文件来源清单。

## 实际验证

- 审查前：原11项测试可通过，但新目录链接回归的两个子场景失败，证明原校验漏检；独立临时index复现manifest mode120000时CLI exit0，真实项目index未改。
- 修复后：`python3 -B -m unittest discover -s tests -p test_verify_upstream.py -v`，16项通过。新增5项方法，包含子场景，验证两个缺陷和index/history不依赖当前工作树。
- `python3 -B tools/verify_upstream.py`：New API／CPR总计4,914文件、54,206,232字节仍通过，源tree与import tree未变。
- 来源/范围子代理独立读取起点Git blob，确认路径、模式、大小、SHA256、许可副本和必需构建输入；源目录从首次导入到审查起点没有修改。
- 工作树和暂存区（`--index`）以及已验证历史提交（`--git-ref dc5263120fd97ee92ddc39e947b55b4ee3bc9a82`）分别核对，确保三种模式不会混用文件来源。最终结果在[E013](../VALIDATION.md#e013-x02审查修订)记录。所有命令只运行自有工具和Git/Python fixture。
- x02-audit-code 对修复作只读复核，确认两个实证问题已关闭，index/historical只读Git对象的边界保持；未扩大到全仓业务审计。

## 改动与未验证边界

只有根目录的校验工具、一个现有测试文件及本项目说明/审查记录改变。保留原始来源pins、两服务源码、许可文本、Mihomo元数据。Git index/historical只依赖被选中的Git对象；不把工作树目录检查误套到它们。

X03仍为pending；未做应用构建、CI、依赖安装、业务登录、OAuth、订阅解析、实际代理和公网出口验证。不对供应商动态IP或源代码全部安全性作承诺。PR继续待审、未合并，不部署。

提交使用同一工作分支普通快进，不强推。先核对远端仍包含审查起点，再推送修订并独立读回分支/PR SHA。发生不确定传输时先查ref，再决定续推；不凭本地成功报告远端完成。
