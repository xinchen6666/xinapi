# X04 本机五服务运行

在单机 Linux 的仓库根目录执行。Docker Engine 与 Compose v2 是前置条件。[根 Compose](../compose.yaml)只发布 New API 到宿主回环，CPR 运行一个副本；两个应用各自使用独立的 PostgreSQL 数据库角色与一套 Redis。

## 准备镜像和配置

已有应用镜像时，用 `XINAPI_NEW_API_IMAGE` 和 `XINAPI_CPR_IMAGE` 指定本机标签。[公开配置示例](../.env.example)列出参数。需要自行构建时按[构建说明](BUILD.md)依次构建两套镜像，再确认标签与 Compose 配置一致。

数据服务使用 PostgreSQL 18 bookworm、Redis 8 alpine；本机已有镜像也可通过 `XINAPI_POSTGRES_IMAGE`、`XINAPI_REDIS_IMAGE` 指定。

```bash
chmod 0755 deploy/compose/init-db.sh
chmod 0644 deploy/compose/cpr-entrypoint.sh services/codex-proxy-rs/deploy/config.example.yaml
python3 -B tools/init_compose.py
docker compose config --quiet
python3 -B tools/start_compose.py --timeout 120
```

`chmod` 只面向公开脚本、模板，尤其适用于 umask 为 077 的检出。生成器按服务写入 `.runtime/x04/`，目录 0700，文件 0600。自选目录时同步设置 `XINAPI_CONFIG_DIR` 和生成器的 `--directory`。复用已有完整配置，缺项时停止并提示。展开的 Compose 配置和 `docker inspect` 环境段含有秘密，仅供部署者本地使用。

[启动入口](../tools/start_compose.py)以两应用实际配置连接各自数据库、认证各自 Redis，并检查 New API `/api/status` 和 CPR `/readyz`。所有条件成功后 exit 0；依赖延迟时在总时限内继续等待，初始化缺项或超时则 exit 1，原配置和命名卷保留。输出仅含服务就绪状态，凭据留在进程内。Compose 的 `--wait` 与容器 running 状态仅代表其现有等待范围；本项目以启动入口的最终结果判断可用。

项目名可由 `--project-name` 或 `COMPOSE_PROJECT_NAME` 指定；`COMPOSE_FILE` 及 `XINAPI_*` 环境继续沿用 Docker Compose 的约定。已启动项目可执行一次性验证：

```bash
python3 -B tools/start_compose.py --wait-only --timeout 120
docker compose ps
```

默认入口 `http://127.0.0.1:3000`。其他宿主回环端口由 `XINAPI_PORT` 指定。本机 HTTP 方式适用于本机调试；完整安装与首次使用走 [单机 HTTPS 安装](INSTALL.md)。

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

## X07 来源与批准版本

「订阅预览 → 来源管理」的读取沿用 `account_egress:read`。预览外部链接、创建、编辑、刷新及批准需 `account_egress:manage_sources`、真实浏览器登录会话和可信来源。Root 默认拥有，普通管理员由 Root 显式授予。OAuth 写入沿用 `account_egress:authorize` 并接受相同来源校验。HTTPS 反代部署配置 `SESSION_COOKIE_SECURE=true` 与精确的 `SESSION_COOKIE_TRUSTED_URL`；本机 HTTP 联调从应用自身入口访问。
New API 的读取响应只包含来源名、版本、节点 UUID、连接版本 ID 与诊断；来源链接只在提交表单中填写。

CPR 首次保存来源时在其持久运行数据目录创建 `subscription.key`（0600、32 字节）。根 Compose 已将该目录
挂载为 cpr-data 命名卷；升级应沿用原数据库与该运行数据卷。来源及连接内容以认证密文保存，
缺少已有部署密钥时，来源读写会提示存储服务错误。节点批准与实际引擎运行是独立步骤，当前账号和出口保持原值。

## X12 管理工作台

从 New API 的「账号与出口」打开来源与节点、上游账号、账号授权及引擎。旧「订阅预览」链接进入工作台的来源与节点页签。

- 先审批来源和节点，再用「验证连接」检查所选连接版本。CPR 使用临时入口访问部署者配置的出口观测服务；账号和路由绑定保持原样。
- 引擎页显示配置上限、实际占用及每个账号或预留的标识。槽位满时可停止指定的账号入口，或回收到期且可回收的预留。停止会结束该入口的连接；从账号详情的受管路由按已发布版本恢复。释放容量后手动继续原来的节点验证或候选准备。
- 来源管理使用 `account_egress:manage_sources`，节点验证、换路与引擎操作使用 `account_egress:manage_egress`，账号启停、分组和额度刷新使用 `account_egress:manage_accounts`。Root 默认拥有管理写权限，其他管理员按动作授权。账号授权沿用 `account_egress:authorize` 和登录会话回执。写操作使用同源登录会话。
- 账号分组是 CPR 上游账号池属性；成员授权映射由 X13 接续。修改分组时系统核对打开表单时的组集合，发生冲突后请重新读取并核对现状。

## X13 成员授权

在 New API 新建 OpenAI 兼容渠道，服务根地址填 `http://cpr:8080`，填入普通占位 Key，并启用需要的模型和 Responses WebSocket。将该渠道 ID 写入私有 New API 服务环境文件的 `XINAPI_CPR_CHANNEL_ID`，随后重启 New API。此渠道采用固定直连目标和服务端成员凭据；代理、请求头覆盖和高级自定义路径保持为空。当前成员通道覆盖 Responses HTTP 与 WS，平台模型目录保持既有实现。

由 Root 或拥有 `account_egress:manage_accounts` 的管理员在用户行菜单打开「上游访问」，显式选择 CPR 账号组；组选项读取沿用 `account_egress:read`，此权限是授组表单的前置。平台注册及平台 Key 创建保持原行为；成员具备有效上游授权后方可经该渠道发送。账号池分组与 New API 平台计费组分别维护。

- 授组与改组将创建专属内部 Key；成员凭据仅服务端使用。成员授权表中的密文依赖服务环境文件里稳定的 `CRYPTO_SECRET`，保留该配置与数据库。支持十六进制编码的 24 或 32 字节密钥。
- 清空选择表示撤权。系统先停止本地准入并禁用原内部 Key，避免空组进入 CPR 的全账号范围。已进入执行的请求沿既有生命周期结束，后续请求和已建 WS 的下一轮重新校验。
- 待确认状态保持停发；通过「核对授权」读取同一操作结果后继续，或明确撤销原操作。成员已停用时仍可走原操作清理。已经发出的创建若远端证据仍缺失，继续保留待确认状态。操作者显式触发后系统沿原操作标识执行，保留条件更新校验。
- 内部 Key 保持禁用记录及其组引用，需删除账号组时先处理对应 Key 的持久引用。真实公网模型调用与长期负载由后续阶段验证。

## X14 Responses 与成员用量

沿 X13 专用成员渠道调用 `/v1/responses`，同步 JSON、SSE 与原生 WS 复用同一成员授权和受管账号出口。WS 原生续接使用同一成员的 `previous_response_id`；工具调用与 `function_call_output` 由客户端按原调用标识回填。

成员用量列表、详情及手机卡片标明数量来源：上游完整报告为「已确认用量」，缺字段时依据已观察内容补算为「估算用量」，证据缺失为「用量待确认」。上游明确的零值保持为零；价格、资金来源与数量精度分别展示。错误记录沿既有 `ERROR_LOG_ENABLED=true` 配置启用，管理员按需要在 New API 服务环境文件设置后重启。

部分输出、取消和失联保留已经观察到的数量及结果，沿既有结算会话完成一次处理。内部成员渠道保持单次发送；结果含糊时由调用者依据已收到内容决定后续动作。

成员概览的首次请求示例按所选平台 Key 的组、模型限制与当前上游授权选择协议：CPR 模型使用本站 `/v1/responses` 和 `input`，普通 OpenAI 渠道沿用 Chat 示例。模型或授权待配置时显示空态，复制操作才读取成员自己的平台 Key。示例用于调用指引，实际请求继续执行数据面准入。

账号详情按稳定节点标识读取当前来源状态。节点从批准来源撤下时，页面说明发送已暂停并指向明确换路；来源状态读取失败时显示待确认。已发布版本及出口 IP 保留历史展示，标为上次记录的观测。来源更新后，同一节点的账号可以继续沿已发布历史版本使用。

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
