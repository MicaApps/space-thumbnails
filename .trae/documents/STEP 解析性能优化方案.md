# 方案 A：使用 truck-stepio 直接解析 STEP 文件

## 1. 验证与原型 (Verification & Prototype)
1.  **检查依赖**：确认 `crates/core/Cargo.toml` 中 `truck-stepio`, `truck-meshalgo`, `truck-polymesh`, `truck-topology` 等依赖的版本和可用性。
2.  **创建测试工具**：在 `crates/core/examples` 或 `tools` 下创建一个独立的 Rust 程序 `test_truck.rs`。
    *   使用 `truck-stepio` 读取指定的 STEP 文件。
    *   使用 `truck-meshalgo` 将几何体离散化 (Tessellation) 为 Mesh。
    *   统计耗时。
3.  **运行测试**：使用您那个 150MB 的 `HLD25-D6-BL-A1.STEP.step` 进行测试，验证是否能在 5 秒内完成。

## 2. 集成到 Core (Integration)
如果测试通过：
1.  **修改 `crates/core/src/lib.rs`**：
    *   取消相关 `truck` 依赖的注释。
    *   实现一个新的函数 `load_step_asset_direct`，替代调用 FreeCAD 的逻辑。
    *   将 `truck` 生成的 Mesh 数据转换为 `AssimpAsset` 或直接转换为 Filament 的 `Renderable`（需要手动构建 VertexBuffer/IndexBuffer）。
    *   *优化路径*：直接转换为 Filament 格式可能比转 Assimp 更快，因为少了一层中间转换。

## 3. 错误处理与回退 (Fallback)
1.  如果 `truck-stepio` 解析失败（例如遇到不支持的实体），我们需要决定是：
    *   回退到 FreeCAD（虽然慢，但总比没有好？或者直接报错？） -> 考虑到 5s 限制，建议直接报错或显示特定图标。
    *   或者尝试降低精度解析。

## 4. 清理 (Cleanup)
1.  移除 `step2obj.bat` 和 `step2obj.py` 相关的代码，精简项目。

现在，我将从**第 1 步**开始：检查依赖并编写测试代码。