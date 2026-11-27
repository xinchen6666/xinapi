# X18 单机发行候选与接手说明

候选标识：`x18-rc1-linux-amd64`。本页交接同版源码、本机可导入的两套应用镜像、对应许可入口和首用范围。三个状态分别记录：源代码合入 `main`、本机候选交接、实际服务部署。远端发行标签、公开镜像仓库和生产部署由所有者分别确认。

## 1. 取得候选材料

| 材料 | 版本与用途 |
| --- | --- |
| `xinapi-x18-rc1-source.tar.gz` | 在合入 [X16](https://github.com/xinchen6666/xinapi/pull/16) 后的 X18 交接分支导出，包含两个应用源码、锁文件、配方、Compose 和本手册。程序源码及应用配方沿 X16 受控实测版本。依赖下载沿锁文件及[构建说明](BUILD.md)进行。 |
| `xinapi-x18-rc1-linux-amd64-images.tar.gz` | 通过 `docker image load` 导入 `xinapi/new-api:x18-rc1-linux-amd64` 与 `xinapi/cpr:x18-rc1-linux-amd64`。New API 是 X16 仓库配方构建；CPR CLI `3.54.2`，Rust 1.99.1 集成编译与仓库运行层、前端。候选标签区别于上游版本和将来的优化发行。 |
| 其余宿主依赖 | Linux amd64、Docker Engine 28.0.0 或更新版本、Compose v2、Python 3、宿主 nginx 及受信 TLS 证书。准备明确版本的 PostgreSQL 18 和 Redis 8 镜像，本轮使用 18.7、8.10.3。Mihomo 由部署者从 [v1.19.33 官方发行](https://github.com/MetaCubeX/mihomo/releases/tag/v1.19.33)取得 `mihomo-linux-amd64-compatible-v1.19.33.gz`，解压后按手册只读挂载。 |

源码包和应用镜像包供本机候选交接，配置、凭据、数据库和卷继续留在各部署者的私有目录。数据服务与 Mihomo 独立取得。已有合规来源的镜像也可按明确标签准备；[CI 镜像结果](BUILD.md#日常-ci)属于构建证据，制品获取以实际交接材料为准。

## 2. 从导入走到成员首个请求

在接手主机将压缩包放至受保护的目录，再从所选工作目录操作：

```bash
docker image load --input /secure/local/xinapi-x18-rc1-linux-amd64-images.tar.gz
tar -xzf /secure/local/xinapi-x18-rc1-source.tar.gz
cd xinapi-x18-rc1
export XINAPI_NEW_API_IMAGE=xinapi/new-api:x18-rc1-linux-amd64
export XINAPI_CPR_IMAGE=xinapi/cpr:x18-rc1-linux-amd64
```

按[安装手册](INSTALL.md)设置其他变量，保留上面的两个候选镜像标签。选择独立 Compose 项目名与私有目录开展首次试用；`tools/init_compose.py` 生成配置，`tools/start_compose.py` 启动五服务。由宿主 nginx 提供唯一团队 HTTPS 入口，浏览器及客户端使用可信证书和精确 Origin。已安装实例按原项目名、配置、密钥及卷使用；X17 专项维护继续暂缓。

部署者确认首次 CPR 管理 Key 签发状态，再显式导入私有文件。通过 New API 创建和批准来源、验证所选节点、完成同会话账号授权、设置账号组和成员上游访问。建立专用渠道后以系统分配的 ID 更新私有环境并重建 New API。成员使用自己的 New API 平台 Key，先完成 HTTPS Responses，再按需运行 SSE 或 WSS。

客户端配置文件以 `0600` 保存成员 Authorization 头，隔离证书可同时配置 `cacert`。下面使用演练中启用的模型；团队使用各自已启用的模型和地址。

```bash
curl --config /secure/local/xinapi-member.curl --fail-with-body \
  --json '{"model":"gpt-4.1-mini","input":"Say hello"}' \
  https://xinapi.example.com/v1/responses
```

同路径的 SSE 请求增加 `"stream":true` 并用 `curl --no-buffer` 读取；WSS 客户端发送 `response.create`，按[安装手册](INSTALL.md#5-以成员身份完成首个请求)核对终态与成员用量。覆盖记录见 [X16](worklog/X16.md)与 [X18](worklog/X18.md)；官方浏览器 OAuth、真实第三方账号及公网节点、持续负载和其他平台仍待专项验证。

## 3. 许可与正式交付边界

| 组件 | 候选随附资料与分发要求 |
| --- | --- |
| New API | [AGPLv3](../services/new-api/LICENSE)、[NOTICE](../services/new-api/NOTICE)、[第三方许可](../services/new-api/THIRD-PARTY-LICENSES.md)随源包保留，并在应用镜像 `/licenses/` 提供。保留可见署名与上游链接，提供实际修改版本对应源码。对外给镜像接收者提供对应源码和构建、安装资料；服务上线时，向网页及仅使用 API 的远程交互用户显著提供同版源码入口。 |
| CPR | [Apache-2.0](../services/codex-proxy-rs/LICENSE)及[构建依赖来源](../third_party/cpr-build-dependencies.lock.json)随源包保留，镜像载有宿主许可。集成镜像采用常规运行层和前端，包含 Debian 运行包及 PG 客户端；正式分发前按实际制品核对宿主修改说明、适用 NOTICE、Rust／前端依赖与静态资产、Debian 和 PostgreSQL 归属。 |
| Mihomo 与研究材料 | [Mihomo GPLv3](../third_party/LICENSES/mihomo-GPL-3.0.txt)及上游入口随源码保留；Mihomo 执行文件由部署者独立取得。如另行转交该二进制，同时准备该制品的对应源码与适用许可材料。Cockpit 仅保留[研究与来源记录](../third_party/cockpit-adoption.md)。 |

仓库按组件记录各自许可，项目自有材料的授权范围由权利人确认。定向传输仍遵循适用许可；源包与镜像包供所有者复核及本机候选试用。正式镜像分发渠道、适用归属材料和优化版本由所有者确认后另行办理。
