## 物联网通信技术

> [!NOTE]
> 1. 本仓库不使用教学文档内 IAR EW8051 + Z-Stack 的主流 CC2530 开发工具链，不保证能够在 IAR 环境内成功编译并烧录
> 2. 实验报告及部分工程使用智能体辅助编写，可能存在不准确的情况，所有内容仅供参考

## 写在前面

截至编辑时，本课程依然使用 CC2530 这个较老的芯片（8051 架构）以及配套的官方工具链。如果你使用 macOS，可惜的是这款芯片并没有像 STM32 那样在 Arm 生态上较为成熟的工具链，其工具链根本无法在 macOS（甚至虚拟机）上运行。

因此，本仓库使用了一些小巧思来完成这些实验。在介绍为什么要这么做之前，不妨先来看看教学文档内的这些工具链到底是什么：

### IAR、Z-Stcak、BasicRF 与 SmartRF

当你开始学习本课程时，我想你已经接触过了 STM32 与 Keil。**IAR** 与 Keil 类似，是一个完整的**第三方** IDE，你可以使用它编辑和烧录程序。**Z-Stcak/BasicRF** 则类似于 STM32 的库函数，调用其提供的函数接口可以快速完成组/入网、收发数据等操作而无需自行实现无线底层协议。而 **SmartRF** 则是一套完整的射频（RF）开发工具，在教学文档中的作用似乎不大。

除了 macOS 无法安装外，这些工具在 Windows 上有什么限制呢？

首先，IAR 需要购买许可证使用。其次，Z-Stack 与 IAR EW8051 工具链深度绑定，其源码中大量使用了 IAR 特有的内联汇编与链接配置等，无法直接使用常见的 C 编译器完成整库编译。

如果你像我一样，不喜欢往电脑里面装破解程序或嵌入式 IDE，可以看看我是怎么做的。

### 如何绕过它们完成实验

> 俗话说，嵌入式开发环境只需要搞定「SDK」+「编译工具」+「烧录工具」就行了（大嘘）

教学文档对应的这三样分别是「Z-Stack & BasicRF」、「IAR EW8051」、「Flash Programmer/IAR & CC Debugger(SmartRF04EB)」。

本项目以「Contiki & BasicRF & ZNP」替代无线协议栈，以「SDCC」替代 IAR 编译器，并提供 Rust 编写的 `xzg` 烧录工具替代 Flash Programmer。各实验的具体组合方式会在对应实验目录中说明。

在仓库根目录构建 `xzg`：

```sh
cargo build --release --manifest-path tools/xzg-rs/Cargo.toml
```

macOS 需要先安装 libusb 和 pkg-config（Homebrew 包名为 `pkgconf`，运行 `brew install libusb pkgconf`）。使用 `tools/xzg-rs/target/release/xzg probe-ti` 检查仿真器与目标芯片，再用 `tools/xzg-rs/target/release/xzg flash-ti <firmware.hex>` 烧录。

> [!NOTE]
> 1. BasicRF 源码也与 IAR 生态绑定，使用 SDCC 无法直接编译
> 2. 在 Apple Silicon 上使用 cc-tool 可能需要安装 Rosetta，推荐使用本项目自带的烧录工具
> 3. 特别感谢 [martin-cao/Breezio-neo](https://github.com/martin-cao/Breezio-neo) 为 CC2530 HAL、BasicRF 迁移 SDCC 以及烧录工具等提供了大量参考实现
