# 应用镜像构建与持续检查

本页覆盖源码到镜像。启动五服务见[运行手册](COMPOSE.md)，早期构建记录见 [X03 工作日志](worklog/X03.md)。

## 日常 CI

[工作流](../.github/workflows/x03.yml)分三种场景：

- **面向 `main` 的 PR**：检查项目文档链接和变更中的凭据特征；按变更范围运行 New API 权限回归、对应前端的类型检查、CPR Rust 格式检查及 Compose 配置解析。
- **推送到 `main`**：构建受影响的应用镜像，执行 New API `--help` 或 CPR `--version`。纯文档变更执行项目检查即可。
- **手动运行**：构建两套应用镜像并执行相同 CLI 检查。镜像在 GitHub 托管 runner 上本地加载，发行与部署另行安排。

检出的应用目录作为镜像构建上下文，各自沿用现有 Dockerfile 与排除规则。依赖和构建工具沿用各自锁定的版本；构建输入与镜像版本检查以正常构建命令及实际 CLI 输出为准。早期导入验证和制品回执工具保留作历史参考，日常工作流使用[项目检查入口](../tools/check_ci.py)。

## 本机构建

先选择所需版本并切到对应源码。本机依次构建应用镜像；按[协作入口](../AGENTS.md#服务器资源与源码保留)预留磁盘和内存，完整双镜像优先走 GitHub Actions。命令在仓库根目录执行，标签可换成部署时约定的值。

```bash
docker buildx build --load --platform linux/amd64 \
  -f deploy/docker/new-api.Dockerfile -t xinapi/new-api:x04-local services/new-api
```

CPR 使用 `runtime` 目标，按所选源码的 `release/version.yaml` 填入 `CPR_VERSION`。先设置以下 `version`，再构建：

```bash
version=3.54.2
recipe_dir=$(mktemp -d)
cp deploy/docker/cpr.Dockerfile "$recipe_dir/Dockerfile"
cp services/codex-proxy-rs/deploy/Dockerfile.dockerignore "$recipe_dir/Dockerfile.dockerignore"
docker buildx build --load --platform linux/amd64 --target runtime \
  --build-arg "CPR_VERSION=$version" \
  -f "$recipe_dir/Dockerfile" -t xinapi/cpr:x04-local services/codex-proxy-rs
docker run --rm --network none --read-only xinapi/cpr:x04-local --version
```

完成后清理本次创建的临时配方目录。版本值随所选源码更新，传给 Compose 的标签和实际构建标签保持一致。

CI 的范围为源码检查、镜像构建与 CLI 验证；数据库、界面、OAuth、模型流和实际出口通过对应业务操作验证。当前构建平台为 Linux amd64。
