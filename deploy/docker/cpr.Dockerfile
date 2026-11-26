# 使用当前 BuildKit 内置 Dockerfile frontend，避免单独拉取外部语法镜像
# 保持上游所有基础镜像版本和 digest；替代仓库必须先验证同一 manifest 摘要
ARG IMAGE_REGISTRY=docker.io

# 备份客户端从固定 PG 镜像取得，与未来 X04 根 Compose 的服务端版本保持一致
# X04 根 Compose 需沿用该 PG pin，升级时同步更新服务端与备份客户端
ARG POSTGRES_IMAGE=${IMAGE_REGISTRY}/library/postgres:18-bookworm@sha256:194e55feb2597b1465dbee81cc9fd8d204566cd2eb42c8d22fd6c931a38781b1

FROM ${IMAGE_REGISTRY}/library/node:26.13.1-bookworm-slim@sha256:5df79989ed685e09f0a98019db4cebfb22fb9df1a53c0161664a011f7ff6b0f1 AS frontend-builder

WORKDIR /app/frontend
COPY frontend/package.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml ./
RUN npm install --global "$(node -p 'require("./package.json").packageManager')" \
    && pnpm install --frozen-lockfile
COPY frontend ./
ARG CPR_WEB_BASE_PATH=/
RUN pnpm build

FROM ${IMAGE_REGISTRY}/library/rust:1.99.1-bookworm@sha256:9181b56c2a21ab1d5e73cffe04fc762426d30aa7fac3c91141dd033f49a7fc8b AS backend-builder

WORKDIR /app/backend
ARG TARGETARCH
# 与其他镜像并行构建时限制 Rust 编译并行度，不改变发布优化或锁文件
ARG CARGO_BUILD_JOBS=2
ENV CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}
ARG CPR_VERSION=
ARG CPR_GIT_SHA=unknown
ARG CPR_BUILD_TIME=unknown
ARG CPR_BUILD_TYPE=release
ENV CARGO_TARGET_DIR=/app/backend/target/${TARGETARCH}
ENV CPR_VERSION=${CPR_VERSION}
ENV CPR_GIT_SHA=${CPR_GIT_SHA}
ENV CPR_BUILD_TIME=${CPR_BUILD_TIME}
ENV CPR_BUILD_TYPE=${CPR_BUILD_TYPE}
# 固定 Git 依赖使用 CLI 传输避免 libgit2 长时阻塞；只影响临时 builder 的网络设置
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
RUN git config --global http.version HTTP/1.1
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/apps ./apps
COPY backend/crates ./crates
COPY backend/migrations ./migrations
COPY release/version.yaml /app/release/version.yaml
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/backend/target \
    cargo build --release --locked --bin codex-proxy-rs \
    && cp "${CARGO_TARGET_DIR}/release/codex-proxy-rs" /tmp/codex-proxy-rs

# 备份客户端来源：X04 根 Compose 服务端必须沿用相同的固定 PG 镜像
FROM ${POSTGRES_IMAGE} AS pg-client

FROM ${IMAGE_REGISTRY}/library/debian:bookworm-slim@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc AS runtime-base

ENV TZ=UTC

# 运行时基础只从 Debian 官方源补 libpq（pg_dump 的动态依赖；ABI 兼容，无需引入第三方
# APT 源）。pg_dump/pg_restore 二进制由 pg-client stage COPY，构建期不依赖运行时安装。
# 更新基础镜像已有的系统包，使固定 digest 的镜像也能获取 Debian 安全更新。
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get upgrade -y \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends ca-certificates curl tzdata libpq5 \
    && install -d /usr/lib/postgresql/18/bin \
    && ln -snf "/usr/share/zoneinfo/${TZ}" /etc/localtime \
    && printf '%s\n' "${TZ}" > /etc/timezone \
    && groupadd --system --gid 10001 cpr \
    && useradd --system --uid 10001 --gid cpr --home-dir /app --shell /usr/sbin/nologin cpr \
    && install -d -o cpr -g cpr \
        /app/bin /app/deploy /app/.runtime/data /app/.runtime/logs /app/web \
    && rm -rf /var/lib/apt/lists/*

COPY --from=pg-client /usr/lib/postgresql/18/bin/pg_dump /usr/lib/postgresql/18/bin/pg_dump
COPY --from=pg-client /usr/lib/postgresql/18/bin/pg_restore /usr/lib/postgresql/18/bin/pg_restore
RUN /usr/lib/postgresql/18/bin/pg_dump --version \
    && /usr/lib/postgresql/18/bin/pg_restore --version

COPY LICENSE /licenses/codex-proxy-rs-LICENSE

WORKDIR /app
# 上游来源与本仓库 OCI revision 分开，避免把根构建适配误当作上游源码改动
LABEL io.xinapi.upstream.source="https://github.com/zyycn/codex-proxy-rs" \
      io.xinapi.upstream.revision="58cbb607505dda4a1cee6e01b9f76e267ad3c261" \
      io.xinapi.upstream.tree="3edc9042af879db4d0d8c5a09c39d2b142f5994c" \
      io.xinapi.upstream.release-tree="d606f1081ffdeb53dbbd9efd89c6edef9427887e" \
      io.xinapi.upstream.version="3.54.2"
ENV CPR_DEPLOYMENT_MODE=docker
ENV PATH="/app/bin:/usr/lib/postgresql/18/bin:${PATH}"

EXPOSE 8080

# 首次升级的历史请求迁移需要启动宽限期
HEALTHCHECK --interval=30s --timeout=5s --start-period=5m --retries=3 \
    CMD curl -fsS http://127.0.0.1:8080/healthz || exit 1

CMD ["/app/bin/codex-proxy-rs"]

FROM runtime-base AS runtime

COPY --from=backend-builder --chown=cpr:cpr /tmp/codex-proxy-rs /app/bin/codex-proxy-rs
COPY --from=frontend-builder --chown=cpr:cpr /app/frontend/dist ./web/dist
USER cpr

FROM runtime-base AS runtime-prebuilt

ARG TARGETARCH
ARG CPR_PREBUILT_DIR=dist/docker

COPY --chown=cpr:cpr ${CPR_PREBUILT_DIR}/linux-${TARGETARCH}/codex-proxy-rs /app/bin/codex-proxy-rs
COPY --chown=cpr:cpr ${CPR_PREBUILT_DIR}/web/dist ./web/dist
COPY --chown=cpr:cpr ${CPR_PREBUILT_DIR}/linux-${TARGETARCH}/plugins/official /app/bin/plugins/official
RUN chmod +x /app/bin/codex-proxy-rs \
    && find /app/bin/plugins/official -type f -exec chmod 0444 '{}' + \
    && chmod 0555 /app/bin/plugins/official \
    && chmod 0755 /app/bin/plugins
USER cpr
