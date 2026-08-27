# Scratch Obfuscator (Rust + Slint)

一个用 Rust 编写的图形化 Scratch 项目（.sb3）混淆工具，支持混淆变量和列表。采用 [Slint](https://slint.dev) 构建原生 GUI，性能优秀且跨平台。

## 特性

- **混淆变量**：将项目中所有变量名替换为随机短字符串。
- **混淆列表**：将项目中所有列表名替换为随机短字符串。
- **混淆角色**（默认隐藏且关闭）：由于此功能还有些问题未能实现，此按钮被隐藏。
- **图形化界面**：简洁直观，支持文件浏览、实时日志输出。
- **多线程处理**：混淆任务在后台执行，界面保持响应。
- **兼容性**：生成的标准 .sb3 文件可直接在 Scratch 3.0 中打开。

## 编译与运行

### 环境要求

- Rust 工具链（1.70 或更高）
- Cargo 包管理器

### 从源码构建

```bash
git clone https://github.com/your-username/scratch-obfuscator-slint.git
cd scratch-obfuscator-slint
cargo run --release