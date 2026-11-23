# X04 本机五服务运行

在单机 Linux 的仓库根目录执行。Docker Engine 与 Compose v2 是前置条件。[根 Compose](../compose.yaml)只发布 New API 到宿主回环，CPR 运行一个副本；两个应用各自使用独立的 PostgreSQL 数据库角色与一套 Redis。

## 准备镜像和配置

已有 X03 镜像时，用 `XINAPI_NEW_API_IMAGE` 和 `XINAPI_CPR_IMAGE` 指定本机标签。[公开配置示例](../.env.example)列出完整参数。新环境先执行：

```bash
python3 -B tools/build_image.py new-api --tag xinapi/new-api:x04-local
python3 -B tools/build_image.py cpr --tag xinapi/cpr:x04-local
```

受限网络可按[构建说明](BUILD.md)选择相应下载通道。基础服务使用 PostgreSQL 18 bookworm、Redis 8 alpine；本机已有镜像也可通过 `XINAPI_POSTGRES_IMAGE`、`XINAPI_REDIS_IMAGE` 指定。

```bash
chmod 0755 deploy/compose/init-db.sh
chmod 0644 deploy/compose/cpr-entrypoint.sh services/codex-proxy-rs/deploy/config.example.yaml
python3 -B tools/init_compose.py
docker compose config --quiet
docker compose up -d postgres redis-newapi redis-cpr
```

`chmod` 只面向公开脚本、模板，尤其适用于 umask 为 077 的检出。生成器按服务写入 `.runtime/x04/`，目录 0700，文件 0600。自选目录时同步设置 `XINAPI_CONFIG_DIR` 和生成器的 `--directory`。复用已有完整配置，缺项时停止并提示。展开的 Compose 配置和 `docker inspect` 环境段含有秘密，仅供部署者本地使用。

首次空卷建库需要几秒钟。执行下面的 SQL，输出 `cpr` 后再启动两个应用；如果 PostgreSQL 仍在初始化，稍后重试同一条：

```bash
docker compose exec -T postgres sh -ec 'PGPASSWORD="$CPR_DB_PASSWORD" psql -X -h 127.0.0.1 -U cpr -d cpr -Atc "select current_database()"'
docker compose up -d --wait
docker compose ps
```

默认入口 `http://127.0.0.1:3000`。其他宿主回环端口由 `XINAPI_PORT` 指定。首次安装由部署者在受信本机完成 New API 初始化，再决定何时开放团队入口。本阶段为本机 HTTP 联调，外部反代与 TLS 配置属于后续交付。

## 两层管理员

1. **团队管理员**使用 New API 的首次安装界面设置账号和口令。该身份管理 New API；团队成员仅使用 New API 入口。
2. **部署者**通过 `docker compose exec -T cpr` 访问 CPR 容器回环 API。CPR 初始用户名为 `admin@cpr.local`，口令来自该实例私有 `cpr.env` 的 `XINAPI_CPR_INITIAL_PASSWORD`。配置渲染在容器的私有 tmpfs，服务以 UID 10001 运行。
3. 取得内部管理凭据的流程：在容器内向 `POST /api/auth/login` 提交 `mode=admin` 和初始凭据，使用该会话调用 `GET /api/admin/settings/admin-api-key`。首次签发使用 `POST /api/admin/settings/admin-api-key/regenerate`，响应的 `data.key` 仅一次返回。将其直接交给本机受保护的后端密钥文件（0600），会话和返回值均按秘密处理；已有密钥时继续使用已有配置。**X04 验证到管理会话登录**；New API 后端消费管理密钥及成员分权属于 X05 以后，当前无需提前签发。CPR 管理端口始终留在容器网络。

## 数据卷和停止

PG18 卷挂在 `/var/lib/postgresql`。专用初始化身份建立 `newapi` 与 `cpr` 两个库，每个同名普通角色只连接自己的应用库。两套 Redis 有独立口令、网络和 AOF 卷。配置目录和 Compose 项目名与该套数据卷配套，后续启动沿用原值。

```bash
docker compose down
docker compose up -d --wait
```

使用常规 `down` 保留命名卷。首次初始化中断时，PostgreSQL 启动脚本会识别已有数据目录缺少完成标记并拒绝向应用提供半套数据库；先停止该项目，保留卷和原配置并检查初始化错误。全新演练可指定独立项目名和端口。应用接入真实订阅、OAuth 及出口策略分别由后续任务交付。
