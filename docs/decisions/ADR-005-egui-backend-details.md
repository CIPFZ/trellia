# ADR-005：egui 后端细节补充

- 编号：ADR-005
- 标题：trellia-egui / RunConfig 中 SPEC.md 未写清的细节
- 日期：2026-10-13
- 状态：已接受

## 背景与决策

### 1. `RunConfig` 的定义位置与字段语义（SPEC.md 11.1）

`RunConfig` 被后端使用，而 `trellia-egui` 不能依赖门面（会成环）。

决策：`RunConfig` 定义在 `trellia-egui`，门面以 `pub use` 重新导出；`run_egui` 是对
`trellia_egui::run` 的转发。字段语义：

- `title` / `initial_window_px` / `vsync`：窗口参数。`initial_window_px` 按 egui 逻辑点（point）解释，
  与操作系统 DPI 缩放一致。
- `logical_size`：供组装 App 时传给 `GfxPlugin::new(cfg.logical_size)`。`run_egui` **不自动添加**
  `GfxPlugin`（插件在 `run_egui` 之前已构建，此时再添加会让其 Render 清屏系统排在游戏 Render
  系统之后，清掉游戏绘制）。`run_egui` 校验 App 中存在 `Viewport` 且 `logical_size` 一致，否则返回错误。
- `asset_root`：同理供组装 `AssetsPlugin { root: cfg.asset_root.clone() }` 使用；`run_egui` 不读取。

`RunConfig` 实现 `Default`（标题 "Trellia"、1280 × 720、vsync 开、`asset_root = None`）。

### 2. 坐标

`FrameInput` 中的窗口尺寸与鼠标坐标使用物理像素（egui 点 × `pixels_per_point`）；绘制时
`Viewport::logical_to_px` 的结果再除以 `pixels_per_point` 转回 egui 点。

### 3. 鼠标与键盘的归属

egui UI（场景 `ui`、调试面板）与游戏共享指针。决策：指针位于 egui 区域之上
（`Context::is_pointer_over_egui`）或 egui 正在使用指针（拖动滑杆）时，鼠标**按下**事件不转发给游戏；
释放事件总是转发（保持按下状态一致）。egui 有焦点控件（文本框）时不转发键盘事件。系统按键重复被过滤。

### 4. 渲染器

eframe 关闭默认 feature，只启用 `default_fonts` / `glow` / `wayland` / `x11`：glow 依赖少、
可在无 GPU 环境用 Mesa 软件渲染运行（CI、Xvfb）。

### 5. 窗口标题与帧耗时（SPEC.md 13 M3“标题栏显示帧耗时”，UC-70）

游戏层只接触 `World`，无法直接改窗口标题，也无法测量后端的 CPU 帧耗时。

决策：`trellia-egui` 插入资源

```rust
pub struct WindowStatus { pub title: String, pub cpu_frame_ms: f32 }
```

`cpu_frame_ms` 为 eframe 统计的上一帧整帧 CPU 耗时（`eframe::Frame::info().cpu_usage`：`App::frame`、
DrawList 转换、UI、tessellation 与绘制提交，不含 vsync 等待），最近 60 帧均值；`title` 被修改后后端在帧末更新窗口标题。只有 egui 后端插入该资源，
游戏层应以 `resources.get_mut::<WindowStatus>()` 可选访问。

### 6. 字体

v1 示例文字只用 ASCII，使用 egui 默认字体；不在仓库中加入 CJK 字体文件。若之后示例需要中文，
另立 ADR 引入带 LICENSE 的开源字体并在后端 `setup_fonts` 中加载。

### 7. 纹理与批处理

纹理在首次遇到某 `Handle<Image>` 时上传（`TextureOptions::NEAREST`）。连续的、同一纹理的精灵
命令合并为一个 `egui::Mesh`，以满足 1000 精灵的帧耗时预算。

## 后果

- 后端可在 Xvfb + Mesa 下运行，便于自动化冒烟检查。
- `WindowStatus` 是后端专属资源；headless 下不存在。

## 替代方案

- （1）`run_egui` 自动添加 `GfxPlugin`：插件顺序错误导致绘制被清空，否决。
- （5）在帧耗时上只用 `Time::real_dt`：那是帧间隔（受 vsync 限制约 16.7 ms），不能反映 CPU 耗时，否决。
