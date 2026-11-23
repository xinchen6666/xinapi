# X03 build adapter for the unchanged services/new-api source tree.
# Transport and compiler concurrency are configurable; versions and digests stay pinned.
ARG IMAGE_REGISTRY=docker.io

FROM ${IMAGE_REGISTRY}/oven/bun:1.4.4@sha256:1d22548f177f4ff38e0c15baa3c2c46c6c2060ba2d1bc573bb4720b407548f1d AS builder

WORKDIR /build/web
COPY web/package.json web/bun.lock ./
RUN bun install --frozen-lockfile
COPY ./web ./
COPY ./VERSION /build/VERSION
RUN DISABLE_ESLINT_PLUGIN='true' VITE_REACT_APP_VERSION=$(cat /build/VERSION) bun run build

FROM ${IMAGE_REGISTRY}/library/golang:1.26.10-alpine@sha256:9ce231b28998b32e116927c61aa5c406183061d4f3f2d61b292c88e7f6bdf7ca AS builder2
ENV GO111MODULE=on CGO_ENABLED=0 GOWORK=off

ARG TARGETOS
ARG TARGETARCH
ENV GOOS=${TARGETOS:-linux} GOARCH=${TARGETARCH:-amd64}
ENV GOEXPERIMENT=greenteagc
# Bound compiler concurrency when gateway components are built on a small host.
ARG BUILD_JOBS=2
ARG GOPROXY=https://proxy.golang.org,direct
ENV GOMAXPROCS=${BUILD_JOBS} GOPROXY=${GOPROXY}

WORKDIR /build

ADD go.mod go.sum ./
ADD relaykit/go.mod ./relaykit/go.mod
ADD tokenkit/go.mod ./tokenkit/go.mod
RUN go mod download

COPY . .
COPY --from=builder /build/web/dist ./web/dist
RUN go build -p ${BUILD_JOBS} -pgo="$(go list -m -f '{{.Dir}}' github.com/MoeForge/moejs)/default.pgo" -ldflags "-s -w -X 'github.com/QuantumNous/new-api/common.Version=$(cat VERSION)'" -o new-api

FROM ${IMAGE_REGISTRY}/library/debian:bookworm-slim@sha256:f06537653ac770703bc45b4b113475bd402f451e85223f0f2837acbf89ab020a

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tzdata libasan8 wget \
    && rm -rf /var/lib/apt/lists/* \
    && update-ca-certificates

COPY --from=builder2 /build/new-api /
COPY LICENSE NOTICE THIRD-PARTY-LICENSES.md /licenses/
LABEL org.opencontainers.image.source="https://github.com/QuantumNous/new-api" \
      io.xinapi.upstream.revision="8542c9d6490b4330fe18160930f81ae7428bc777" \
      io.xinapi.upstream.tree="11dbbb8a51210a38b84d9f9358c56c9d0415f5e8"
EXPOSE 3000
WORKDIR /data
ENTRYPOINT ["/new-api"]
