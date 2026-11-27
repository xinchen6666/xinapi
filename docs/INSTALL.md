# 单机 HTTPS 安装与首次使用

本手册适用于 Linux amd64、Docker Engine 28.0.0 或更新版本、Compose v2、Python 3 和宿主 nginx。Docker 沿用默认 bridge/NAT 隔离策略，New API 只发布宿主回环 HTTP，团队统一使用 nginx HTTPS；CPR、PostgreSQL 和两套 Redis 留在容器网络。[Docker 回环端口说明](https://docs.docker.com/engine/network/port-publishing/#publishing-ports)解释了版本要求。已有安装继续沿用原项目名、私有配置目录和数据卷。

## 1. 准备明确版本的本地镜像

```bash
git clone https://github.com/xinchen6666/xinapi.git
cd xinapi
```

切换到本次使用的发布版本或已审阅的源码分支。先按 [镜像构建说明](BUILD.md#本机构建) 依次构建两套应用镜像，或从受信渠道把指定版本载入本机。下例应用标签 `x16-local` 指本机演练产物。准备与该部署配套的 PostgreSQL 18 和 Redis 8 本地镜像；已有构建及真实服务版本见 [X16 日志](worklog/X16.md)。安装 Mihomo v1.19.33 Linux amd64 compatible 可执行文件到宿主固定位置，权限应允许容器 UID 10001 读取和执行。来源及许可见 [上游清单](UPSTREAM.md)。

## 2. 创建私有配置并启动五服务

把示例路径、项目名、端口和标签换成本机值。使用绝对路径保存配置及 Mihomo，源码目录可以独立更换。

```bash
export COMPOSE_PROJECT_NAME=xinapi-home
export COMPOSE_FILE=compose.yaml:deploy/compose/mihomo.yaml
export XINAPI_CONFIG_DIR=/var/lib/xinapi/config
export XINAPI_MIHOMO_PATH=/opt/xinapi/bin/mihomo
export XINAPI_NEW_API_IMAGE=xinapi/new-api:x16-local
export XINAPI_CPR_IMAGE=xinapi/cpr:x16-local
export XINAPI_POSTGRES_IMAGE=postgres:18-bookworm
export XINAPI_REDIS_IMAGE=redis:8-alpine
export XINAPI_PORT=3000

chmod 0644 deploy/compose/cpr-entrypoint.sh services/codex-proxy-rs/deploy/config.example.yaml
chmod 0755 deploy/compose/init-db.sh
python3 -B tools/init_compose.py --directory "$XINAPI_CONFIG_DIR"
```

在私有 `new-api.env` 文件中设置以下单值，每项保留一个定义。可信地址填浏览器实际使用的 HTTPS Origin，若用非默认端口须同时填写端口。生成器自带 `TRUSTED_PROXIES=none`：New API 此时以实际 TCP 对端识别客户端，成员 IP 限制按该地址工作。若确需识别成员原始 IP，只信任已确认的直接反代地址，并由反代覆盖客户端提供的 IP 头。

```dotenv
SESSION_COOKIE_SECURE=true
SESSION_COOKIE_TRUSTED_URL=https://xinapi.example.com
TRUSTED_PROXIES=none
```

```bash
docker compose config --quiet
python3 -B tools/start_compose.py --timeout 180
```

[启动入口](../tools/start_compose.py) 报告五服务就绪后继续。[基础 Compose](../compose.yaml) 只将 New API 绑定到 `127.0.0.1`，[Mihomo 覆盖文件](../deploy/compose/mihomo.yaml) 将指定制品只读挂入 CPR。应用镜像从本机读取；镜像标签变更和服务环境文件变更均通过 `docker compose up -d` 应用，需要时显式重建相应容器。

## 3. 建立唯一 HTTPS 入口

把 [nginx 示例](../deploy/compose/nginx-https.conf.example) 安装为 nginx 的一个站点，包含在 `http` 作用域。替换 `server_name`、证书文件、监听地址及 `proxy_pass` 的宿主回环端口。先以仅部署者可达的方式开启站点，完成首次管理员设置后再开放团队访问。客户端应信任证书的签发链。

先运行 `nginx -t`，再按宿主现有的 nginx 管理方式加载站点。示例在同一个 `/v1/responses` 路径支持 SSE 与 WebSocket：流关闭代理缓冲，升级头沿 HTTP/1.1 转给应用，上游连接失败由客户端明确处理。New API 的回环 HTTP 仅服务宿主反代。CPR 管理端口、PG 及 Redis 继续留在 Docker 内网。

## 4. 完成首次管理员操作

1. 从 HTTPS 入口打开首次设置，创建 New API 管理账号并登录。按需在系统设置中填入实际 HTTPS 站点地址。
2. 部署者在 CPR 容器内按 [两层管理员](COMPOSE.md#两层管理员) 的步骤签发首次管理 Key；使用已有 Key 时沿用原私有文件。显式导入部署者已保存的 Key，目标文件权限为 0600：

   ```bash
   test ! -e "$XINAPI_CONFIG_DIR/new-api-secrets/cpr-admin.key" &&
     install -m 0600 /secure/local/cpr-admin.key \
       "$XINAPI_CONFIG_DIR/new-api-secrets/cpr-admin.key"
   ```

   New API 从只读挂载重新读取私有 Key；页面重试即可核对导入结果。已有目标文件时保留它。浏览器只使用 New API 登录会话。
3. 在「账号与出口 → 来源与节点」创建来源，刷新并审阅差异，明确批准节点。对所选节点执行「验证连接」，再从「账号授权」选择该节点开始 OAuth；在同一 New API 登录会话提交回调并读取完成回执。浏览器第三方登录的网络与 CPR 服务端出口分别确认。
4. 使用 CPR 内部管理入口创建一个账号组，例如调用 `POST /api/admin/account-groups/create`，请求体为 `{"name":"Team","color":"#2563EBFF"}`，管理 Key 保持在部署者受保护的调用环境。从 New API 的账号详情将已授权账号加入该组。
5. 在 New API 创建 [专用渠道](COMPOSE.md#x13-成员授权)：OpenAI 兼容、服务根 `http://cpr:8080`、启用所需模型和 Responses WebSocket。填写独立占位 Key，代理和覆盖留空。记录实际渠道 ID，设为 `new-api.env` 的 `XINAPI_CPR_CHANNEL_ID`，再执行：

   ```bash
   docker compose up -d --no-deps --force-recreate new-api
   python3 -B tools/start_compose.py --wait-only --timeout 180
   ```

6. 在用户行的「上游访问」明确授予成员相同 CPR 账号组。成员从自己的账号创建 **New API 平台 Key**；内部 Key 和上游凭据留在服务器。按团队现有规则为成员提供该模型群组及调用额度。

## 5. 以成员身份完成首个请求

以受保护的客户端配置发送 `POST https://xinapi.example.com/v1/responses`，认证使用 `Authorization: Bearer <成员平台 Key>`，正文包含启用的 `model` 和 `input`。先读取非流式结果；设 `stream: true` 后检查首个事件及时到达与 `response.completed`。支持 WebSocket 的客户端连接 `wss://xinapi.example.com/v1/responses`，沿用同一成员认证，发送 `response.create` 并检查终态事件。最后在成员用量页核对请求记录及数量来源。

### 排查顺序

- HTTPS 登录或写入被拒绝：核对浏览器证书信任、`SESSION_COOKIE_SECURE` 和完整可信 Origin，再确认 env 变更已通过重建生效。
- 账号页面服务错误：核对首次管理 Key 的私有文件权限、挂载目录与 CPR 就绪状态。
- 模型返回权限或可用账号错误：检查专用渠道 ID、成员授组、账号同组、批准节点及运行入口。停机后的路由按原发布回执恢复。
- SSE 延迟或 WS 升级失败：检查 TLS 入口及经过的反代层，沿用示例的流缓冲与升级设置，并检查渠道的 Responses WebSocket 开关。

计划停机使用 `docker compose down`，保留命名卷、原项目名和全部私有配置。安装验证与扩大开放范围分开进行；官方浏览器授权、真实公网账号／节点及长期负载按专项范围验证。
