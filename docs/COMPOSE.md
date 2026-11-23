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
python3 -B tools/start_compose.py --build --timeout 120
```

`chmod` 只面向公开脚本、模板，尤其适用于 umask 为 077 的检出。生成器按服务写入 `.runtime/x04/`，目录 0700，文件 0600。自选目录时同步设置 `XINAPI_CONFIG_DIR` 和生成器的 `--directory`。复用已有完整配置，缺项时停止并提示。展开的 Compose 配置和 `docker inspect` 环境段含有秘密，仅供部署者本地使用。

[启动入口](../tools/start_compose.py)以两应用实际配置连接各自数据库、认证各自 Redis，并检查 New API `/api/status` 和 CPR `/readyz`。所有条件成功后 exit 0；依赖延迟时在总时限内继续等待，初始化缺项或超时则 exit 1，原配置和命名卷保留。输出仅含服务就绪状态，凭据留在进程内。Compose 的 `--wait` 与容器 running 状态仅代表其现有等待范围；本项目以启动入口的最终结果判断可用。

项目名可由 `--project-name` 或 `COMPOSE_PROJECT_NAME` 指定；`COMPOSE_FILE` 及 `XINAPI_*` 环境继续沿用 Docker Compose 的约定。已启动项目可执行一次性验证：

```bash
python3 -B tools/start_compose.py --wait-only --timeout 120
docker compose ps
```

默认入口 `http://127.0.0.1:3000`。其他宿主回环端口由 `XINAPI_PORT` 指定。首次安装由部署者在受信本机完成 New API 初始化，再决定何时开放团队入口。本阶段为本机 HTTP 联调，外部反代与 TLS 配置属于后续交付。

## 两层管理员

1. **团队管理员**使用 New API 的首次安装界面设置账号和口令。该身份管理 New API；团队成员仅使用 New API 入口。
2. **部署者**通过 `docker compose exec -T cpr` 访问 CPR 容器回环 API。CPR 初始用户名为 `admin@cpr.local`，口令来自该实例私有 `cpr.env` 的 `XINAPI_CPR_INITIAL_PASSWORD`。配置渲染在容器的私有 tmpfs，服务以 UID 10001 运行。
3. 在容器内向 `POST /api/auth/login` 提交 `mode=admin` 和初始凭据，使用该会话调用 `GET /api/admin/settings/admin-api-key` 查看签发状态。首次签发使用 `POST /api/admin/settings/admin-api-key/regenerate`，响应的 `data.key` 仅一次返回。将其直接保存到本机配置目录的 `new-api-secrets/cpr-admin.key`（0600）；会话、响应和文件均按秘密处理，已有 Key 时沿用已有私有文件。CPR 管理端口始终留在容器网络。

## X05 账号与出口入口

[配置生成器](../tools/init_compose.py)创建 `new-api-secrets/` 私有目录（0700）；已有 X04 配置也可再次运行生成器，原凭据保持。部署者将上节取得的 CPR 管理 Key 写入该目录的 `cpr-admin.key`，设置文件权限为 0600。下例的源文件由部署者选取，内容保持在受信本机：

```bash
python3 -B tools/init_compose.py --directory "${XINAPI_CONFIG_DIR:-./.runtime/x04}"
install -m 0600 /secure/local/cpr-admin.key \
  "${XINAPI_CONFIG_DIR:-./.runtime/x04}/new-api-secrets/cpr-admin.key"
python3 -B tools/start_compose.py --timeout 120
```

[根 Compose](../compose.yaml)将整个目录以只读方式挂到 New API 的 `/run/xinapi`，由 `XINAPI_CPR_ADMIN_KEY_FILE=/run/xinapi/cpr-admin.key` 指定后端消费位置。Key 只由 Go 后端读取，使用固定 CPR 内部地址访问账号和出口列表。缺少有效私有文件或 CPR 暂时离线时，页面展示服务错误并提供重试；Key 文件更新后，后续请求直接读取新值。

管理员登录 New API 后，从「账号与出口」查看两个只读列表。入口同时要求管理员角色与 `account_egress:read`，个人访问令牌也沿用同名 scope。账号展示状态、套餐与出口 ID；出口展示关联账号数和认证配置状态。代理地址、出口 IP、邮箱、上游身份和凭据保持在 CPR 内部。

## 数据卷和停止

PG18 卷挂在 `/var/lib/postgresql`。专用初始化身份建立 `newapi` 与 `cpr` 两个库，每个同名普通角色只连接自己的应用库。两套 Redis 有独立口令、网络和 AOF 卷。配置目录和 Compose 项目名与该套数据卷配套，后续启动沿用原值。

```bash
docker compose down
python3 -B tools/start_compose.py --timeout 120
```

使用常规 `down` 保留命名卷。首次初始化中断时，PostgreSQL 启动脚本会识别已有数据目录缺少完成标记并拒绝向应用提供半套数据库；按下节在原卷完成补救。全新演练可指定独立项目名和端口。应用接入真实订阅、OAuth 及出口策略分别由后续任务交付。

## 在原卷补全首次初始化

适用状态为已有 PG_VERSION、缺少 `.xinapi-databases-ready`，常见阶段为只建立角色、只建立一个应用库，或授权已提交而完成标记尚待写入。原配置目录、项目名和镜像继续沿用。先检查本机受保护的 PostgreSQL 启动日志，修正公开挂载文件的读取权限等原始原因。

先停止应用和普通数据库容器，再以原管理身份启动仅接受 Unix socket 的维护容器；该容器使用同一 PostgreSQL 卷，保持宿主端口为空：

```bash
docker compose stop
chmod 0644 deploy/compose/complete-db-init.sh
docker compose run --no-deps -d --name xinapi-pg-maintenance \
  --entrypoint docker-entrypoint.sh \
  --volume "$PWD/deploy/compose/complete-db-init.sh:/etc/xinapi/complete-db-init.sh:ro" \
  postgres postgres -c listen_addresses=
```

以下命令输出 `1` 后继续；启动中的连接失败可稍后重试：

```bash
docker exec xinapi-pg-maintenance sh -ec \
  'PGPASSWORD="$POSTGRES_PASSWORD" psql -X -w -U "$POSTGRES_USER" -d postgres -Atc "SELECT 1"'
```

[补全脚本](../deploy/compose/complete-db-init.sh)先检查已有角色权限和数据库归属，再创建缺失角色或数据库。已启用角色使用原配置实际认证，认证成功后保留已有口令；NOLOGIN 角色在两库授权齐全后使用原配置口令启用。最后验证两个角色各自登录成功、跨库访问被拒绝，随后写入完成标记。遇到已有角色权限、库归属或原口令冲突时，按提示核对原部署配置与对象，继续保留该卷；完成标记由脚本在验证成功后写入。

```bash
docker exec --user postgres xinapi-pg-maintenance \
  sh /etc/xinapi/complete-db-init.sh
```

确认输出 `initialization completed on the retained volume` 后，停止本节建立的维护容器，并通过正式启动入口验证五服务：

```bash
docker stop xinapi-pg-maintenance
docker rm xinapi-pg-maintenance
python3 -B tools/start_compose.py --timeout 120
```

补全操作保留卷中的已有表和记录。完成标记已存在时，脚本提示通过正式启动入口验证服务。最后使用原管理员口令和原应用凭据验证登录，并读取原业务记录确认数据保持。
