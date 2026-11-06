# WebDAV 客户端（Tauri 2 + Vite + TypeScript）

简单但功能齐全的 WebDAV 桌面客户端：多连接管理、浏览目录、上传/下载、重命名、删除、新建文件夹、在线预览。

## 🚀 CI/CD 发布

本仓库使用 GitHub Actions 自动化构建与发布：

- **触发条件**：`push` 到 `main` 分支、`v*` 标签、`pull_request`、`workflow_dispatch`
- **构建**：`windows-latest` 上安装 Node 22 + Rust stable，执行 `npm ci` → `npm run build` → `cargo check` → `npx tauri build`
- **产物**：`src-tauri/target/release/bundle/` 上传为 workflow artifact
- **发布**：当 push 的是 `v*` tag 时，自动创建 GitHub Release 并上传产物（`softprops/action-gh-release`，自动生成发布说明）

```bash
# 发布新版本：
git tag v1.0.0
git push origin v1.0.0
```

## 技术栈

- **前端**: Vite 6 + TypeScript，`@tauri-apps/api` + 官方 `plugin-dialog` / `plugin-opener`
- **后端**: Tauri 2 + reqwest（rustls）+ quick-xml 解析 PROPFIND、serde
- **架构**: Rust 端实现 WebDAV 协议（PROPFIND / MKCOL / DELETE / MOVE / PUT / GET），前端只发 `invoke` 命令；浏览器不直接发请求，避免 CORS

## 项目结构

```
src-tauri/
  src/lib.rs        # Builder + invoke_handler 注册
  src/main.rs       # 入口（!windows_subsystem）
  src/dav.rs        # WebDAV 核心：连接、列表、建夹、删除、重命名、上传下载、配置持久化
  src/preview.rs    # 预览命令（base64 返回前端）
  capabilities/main.json
  icons/            # 由 Pillow 生成的占位图标，可替换
index.html / vite.config.ts / tsconfig.json
src/
  main.ts           # 入口：组装页面、状态（cfg/currentPath/entries）
  lib/api.ts        # 全部 invoke 封装
  lib/types.ts      # DavConfig / FileEntry / PreviewData
  lib/format.ts     # 格式化大小/日期/图标
  components/
    ConnectionBar.ts # 顶部连接条（地址/用户名/密码 + 测试连接）
    Toolbar.ts       # 面包屑 + 搜索 + 新建/上传/刷新
    FileTable.ts     # 文件表格（排序、双击打开）
    Progress.ts      # 上传进度条（监听 upload-progress 事件）
    PreviewModal.ts  # 图片/视频/音频/PDF/文本预览弹窗
  style.css
```

## 已实现功能

| 模块 | 功能 |
|------|------|
| 连接 | 服务器地址 + 用户名 + 密码（HTTP Basic），PROPFIND Depth:0 测试连通性，配置持久化到本地（`data_dir/webdav-client/webdav_config.json`） |
| 浏览 | PROPFIND Depth:1 列目录，文件夹优先，面包屑导航，双击进入子目录 |
| 搜索/排序 | 前端按名称过滤；按名称/大小/时间排序（点击表头） |
| 新建文件夹 | MKCOL |
| 删除 | DELETE（应用内二次确认） |
| 重命名 | MOVE + Destination 头 |
| 下载 | GET → 保存到系统「下载」目录 |
| 上传 | 文件选择对话框（多选）→ 分块 PUT + 进度事件；同时提供 `upload_file`（bytes） |
| 预览 | 图片/视频/音频/PDF/文本（base64）弹窗 |

## 启动方式

```bash
# 安装依赖
npm install

# 开发模式（启动 Vite + Tauri）
npm run tauri dev

# 构建
npm run build        # 仅前端
npx tauri build      # 含 Rust 的完整打包
```

## WebDAV 命令 → Tauri 命令 对应关系

| WebDAV | Tauri command | 说明 |
|--------|---------------|------|
| PROPFIND | `test_connection` / `list_dir` | 测试连通性 / 列目录（quick-xml 解析 multistatus） |
| MKCOL | `create_folder` | 新建文件夹 |
| DELETE | `delete_entry` | 删除文件/文件夹 |
| MOVE | `rename_entry` | 重命名（同目录改名） |
| GET | `download_file` / `preview_file` | 下载到本地下载目录 / 内存预览 |
| PUT | `upload_file` / `upload_local_file` | 内存字节 / 本地文件 |

## 说明

- 图标为 Pillow 生成的占位图，替换 `src-tauri/icons/` 下图片并保持 `tauri.conf.json` 的 `bundle.icon` 即可。
- `src-tauri/src/dav.rs` 为核心文件，包含 URL 编码、响应解析、连接持久化等全部逻辑；`preview.rs` 按扩展名判断类型。
