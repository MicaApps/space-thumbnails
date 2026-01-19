## 速度优化方案 (Speed Optimization Plan)

当前程序“卡死”的原因是 STEP 文件包含大量复杂的几何曲面，程序目前是**单线程**逐个计算这些曲面的三角网格（Tessellation），对于 120MB 的大文件，这会非常耗时且无法充分利用您的 CPU 性能。

### 1. 引入并行计算 (Parallel Processing)
*   **措施**: 引入 `rayon` 库。
*   **效果**: 将原本“排队一个一个做”的计算任务，改为“大家一起做”。对于多核 CPU，理论上可以提升数倍速度（例如 8 核 CPU 可能快 5-6 倍）。
*   **代码变更**: 修改 `crates/core/Cargo.toml` 添加依赖，修改 `lib.rs` 使用 `par_iter`。

### 2. 调整计算精度 (Tuning Tolerance)
*   **措施**: 保持或微调三角化精度为 `0.1`（原为 `0.01`）。
*   **理由**: 生成缩略图不需要极高的几何精度，适当降低精度可以大幅减少计算量和内存占用，且肉眼几乎看不出区别。

### 3. 优化合并策略 (Optimize Merging)
*   **措施**: 使用并行归约（Parallel Reduce）的方式合并网格，进一步减少等待时间。

---

**执行步骤**:
1.  修改 `crates/core/Cargo.toml` 添加 `rayon` 依赖。
2.  重构 `crates/core/src/lib.rs` 中的 `load_step_asset` 函数，使用并行迭代器处理 Shells。
3.  运行测试，验证速度提升。
