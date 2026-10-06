# CC2530 SDK

`cc2530/hal` 和 `basicrf` 保存了 Breezio-neo 的 SDCC 固件所用的 CC2530 HAL 与 TI Basic RF 组件。源文件保留了 Texas Instruments 原有的版权声明和芯片专用条款，详见 [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md)。

`cc2530/include/platform.h` 是 Breezio-neo 的编译器适配头文件。使用 SDCC 时，它会包含 MCS-51 工具链提供的 `<cc2530.h>`；该编译器头文件由工具链提供，本仓库不重复复制。编译使用这些组件的程序时，请添加以下头文件搜索路径：

```sh
-I sdk/cc2530/include -I sdk/cc2530/hal -I sdk/basicrf
```

这里提供的是可复用的源码基础，不是完整的板级支持包。Breezio-neo 中的传感器驱动、板卡引脚映射、UART 配置和应用启动代码仍属于具体应用，本仓库未导入这些内容。

导入的上游 HAL 中，`halRfReadMemory()` 和 `halRfWriteMemory()` 仍是返回零的空实现，`halMcuSetLowPowerMode()` 也未实现。使用这些接口前应先检查实现；Basic RF 的收发路径不依赖这两个内存访问接口。
