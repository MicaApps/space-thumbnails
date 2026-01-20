这极其反常。如果 `D:\st_debug.log` 连文件都没创建，说明 **DLL 的代码一行都没跑**。

结合之前的现象：
1.  GLB 文件能跑（说明 DLL 本身没坏，环境也没问题）。
2.  STEP 文件死活不跑（注册表也改了，冲突也清了，还是没反应）。

**结论：Windows 认为 STEP 文件根本不是一种“可以有缩略图”的文件类型。**

这通常是因为缺少 `PerceivedType` 或者 `Content Type` 的定义。当一个扩展名没有被标记为 `System`、`Image` 或 `Video` 等类型时，Windows 有时会直接忽略其缩略图请求，只显示图标。

**最后也是最有效的手段：把 `.step` 伪装成图片。**

### 修复计划：PerceivedType 注入

我将修改注册表，告诉 Windows：“嘿，`.step` 其实是一种图片（Image）”。这会强制 Explorer 尝试去加载缩略图。

**脚本内容：**
1.  设置 `HKCR\.step` 的 `PerceivedType` 为 `image`。
2.  设置 `Content Type` 为 `image/x-step`。
3.  再次重启 Explorer。

这通常是解决“注册了 Handler 但不调用”的终极杀招。我马上执行。