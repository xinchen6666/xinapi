# X03 构建与持续检查

本页只覆盖源码到镜像的构建，不是部署说明。根五服务 Compose 和真实账号集成属于后续任务；当前状态与实际结果见[进度](STATUS.md)、[X03日志](worklog/X03.md)。

## 固定输入

- New API：上游 `8542c9d…`，Bun 1.4.4、Go 1.26.10，原样前端锁文件和3个Go模块。
- CPR：上游 `58cbb607…`，Node 26.13.1、pnpm 12.6.0、Rust 1.99.1，UI包与Cargo Git依赖沿用锁定提交。可选源码联调gitlink不参与正式镜像构建。
- 两个[根构建适配](../deploy/build-targets.json)沿用上游基础镜像完整digest，未降版本或更改业务源码。参数只控制拉取通道、编译并行度和构建身份。
- [CI动作](../.github/actions.lock.json)使用完整提交SHA；检查用的Ruff固定版本及wheel SHA256。Linux amd64为本步验证平台，arm64尚未验证。

## 从已提交快照构建

需要Python 3.12+、Docker Engine和Buildx。检查工具不会安装Docker或更改全局daemon配置。

```bash
python3 -B tools/build_image.py new-api --revision HEAD
python3 -B tools/build_image.py cpr --revision HEAD
```

[构建入口](../tools/build_image.py)从指定提交读取配方和source tree，在临时目录解包Git快照，不发送未提交的`.env`、研究缓存或根Git目录。Dockerfile、ignore policy必须是Git普通文件。默认只`--load`到本机，不推送镜像或启动服务。

每次构建保存独立JSON回执、BuildKit元数据和本地日志到忽略的`.runtime/`，记录Git提交、source tree、配方/ignore摘要、镜像ID、平台和实际退出码。指定`--output`时必须是该目录下尚不存在的JSON文件，避免覆盖旧证据。成功必须取得有效BuildKit导出元数据，将本次exported digest与加载的Docker对象绑定，并确认实际OS/架构、OCI source和revision符合请求；缺失元数据、错平台或残留tag会失败。兼容经典Docker的config ID和containerd的OCI index ID，两者仍须由同一次导出摘要锚定。默认tag包含提交与平台，减少交叉覆盖。

## 受限网络的显式替代通道

本机构建原Docker Hub路径发生连接超时。检查可达镜像通道时，先对完整manifest字节重算SHA256并匹配原digest，实际拉取仍使用同一digest；不是找一个相似tag替代原版本。

```bash
python3 -B tools/build_image.py new-api --registry docker.m.daocloud.io --go-proxy https://goproxy.cn,direct
python3 -B tools/build_image.py cpr --registry docker.m.daocloud.io
```

镜像默认来自Docker Hub，Go默认使用官方proxy。替代通道必须显式选择并进入回执，不能静默fallback。Go module checksum/GOSUMDB和Cargo锁/包校验均保留，不关闭TLS验证。CPR的Git HTTP传输适配仅在临时builder中，不改宿主Git配置。

这种通道替代不保证下载站始终可达，也不是新的可信版本来源。基线源码、锁文件和基础镜像digest是身份依据。APT仍会获取当时的安全更新，因此不承诺不同日期构建的镜像逐字相同；实际镜像用ID/digest标识。

## 项目检查与 CI

```bash
python3 -B -m unittest discover -s tests -p 'test_*.py' -v
python3 -B tools/verify_upstream.py --git-ref 072e434a528bb9295d9c6d4ee6000c91b03485ca
python3 -B tools/check_project.py --git-ref HEAD
```

[工作流](../.github/workflows/x03.yml)在PR和工作分支推送时运行：

1. 项目工具格式/lint、单元测试、固定来源基线、文档/任务结构和凭据特征检查。
2. 按改动路径选择New API、CPR或两个镜像；纯文档更新不重复编译镜像，必要检查失败不能被汇总为通过。
3. 从当前提交快照构建；无网络、只读短命容器只执行帮助/版本命令，不启动监听服务、数据库或OAuth。
4. 只上传构建元数据JSON，不上传完整日志、镜像、账号配置或缓存；权限仅`contents: read`。

凭据特征检查覆盖全部Git文件。对已审查上游的合成样例，只认可固定基线同路径、相同内容与原有出现次数，新增/复制/重复匹配会失败；不整目录豁免services。它不是穷举秘密或安全审计，不要将真实秘密写入Git。

成功编译和帮助/版本退出只能证明构建与可执行性，不代替数据库、界面、OAuth、路由、SSE/WS或真实出口验收。CLI/CI失败必须保留具体日志和退出码，不用替换锁文件、屏蔽错误或删除前端让构建假绿。
