# 当前交接

小团队非商业使用，单机Docker Compose。新会话先核对[状态](STATUS.md)、本页和实际Git分支，再查询PR当前head。只把查询时的状态当事实。

## X02 当前交付

固定 New API、CPR 源码、来源清单、验证器和回归测试已推至工作分支 [`task/X02-source-baseline`](https://github.com/xinchen6666/xinapi/tree/task/X02-source-baseline)，位于 [Draft PR #1](https://github.com/xinchen6666/xinapi/pull/1)，base `main`。

最后一次状态查询记录在[X02日志](worklog/X02.md)：PR head 为 `bfd56a6c26db28599dc2194095675ec9839296bf`，PR仍Draft/open/未合并，未见CI status/check-runs。PR head root tree与本地X02目标tree相同，New API及CPR导入tree各自匹配来源清单。普通GitHub克隆超时；内容已用GitHub commit/tree API复核，不能把API树对照称为一次成功克隆。

X02没有构建/运行New API、CPR、Mihomo或Compose；没有安装运行时依赖，没有真实账号、OAuth、订阅或公网出口测试。Cockpit业务实现未复制，Mihomo二进制未下载。

## 下一步

维护者审阅Draft PR #1、确认许可边界及必要检查。若仅合并交接记录，GitHub会生成新的PR head commit SHA；接手时先读GitHub当前branch/PR，不照抄上述可能过时的SHA。未经用户授权不合并main、不部署。PR合入后复核merge SHA并将X02设为done，之后从合并后的main开启[X03基线构建与CI](PLAN.md#x03)。

## 固定来源与误判边界

- New API upstream/import tree：`11dbbb8a51210a38b84d9f9358c56c9d0415f5e8`。
- CPR发布root tree `d606f1081ffdeb53dbbd9efd89c6edef9427887e` 含两个可选UI/plugin源码gitlink；xinapi不存嵌套repo，普通文件import tree为`3edc9042af879db4d0d8c5a09c39d2b142f5994c`。manifest分别锁这两个tree与省略SHA。
- Cockpit CC BY-NC-SA与Apache/AGPL的组合兼容性未确认，X02只保留逐文件采用清单，没有复制实现。Mihomo `.gz`摘要来自release API；未下载、解压、签名核验或执行。
- 五服务Compose、依赖安装、两应用构建仍属X03及以后。固定源码存在不表示可构建或已部署。
- CPR强制出口和成员权限是后续需求；账号停用不等于后台token刷新停发，禁用代理可能回到默认网络。

凭据不写入源码、日志、PR或交接。当前无Docker进程、数据库、账号或订阅数据运行。
