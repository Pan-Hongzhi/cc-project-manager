# CC Project Manager（Windows）

管理 Claude Code 在 `%USERPROFILE%\.claude` 下的项目状态：看清项目清单、活跃度、分类空间与 token 用量。

当前版本：M1（只读）。删除、迁移、导入导出在后续版本。

## 开发

- 依赖：Rust stable (MSVC)、Node 24、VS 2022 C++ 桌面工作负载
- `cargo test` 运行核心库测试；`npm test` 运行前端测试
- `npm run tauri dev` 启动开发窗口；`npm run tauri build` 打包

## 文档

- 需求：`CC Project Manager（Windows）需求规格.md`
- 设计：`CC Project Manager（Windows）软件设计.md`
- 计划：`docs/superpowers/plans/`

## 验收

- M1 只读真机验收记录：`docs/acceptance/2026-09-22-m1-readonly.md`
