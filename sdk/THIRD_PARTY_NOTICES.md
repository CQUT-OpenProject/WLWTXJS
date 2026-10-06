# SDK 第三方声明

## Breezio-neo 编译器适配头文件

[`cc2530/include/platform.h`](cc2530/include/platform.h) 复制自 Breezio-neo 的 `sensor/source/platform.h`，采用 Apache-2.0 许可证。许可证全文见 [`licenses/Breezio-neo-LICENSE`](licenses/Breezio-neo-LICENSE)。

## Texas Instruments CC2530 Basic RF 与 HAL

以下文件复制自 Breezio-neo 的 `sensor/source/` 目录，源自 Texas Instruments CC2530 Basic RF、HAL 和时钟代码，并保留文件内的 TI 版权声明及许可条款：

- `basicrf/basic_rf.c` 和 `basicrf/basic_rf.h`
- `cc2530/hal/clock.c` 和 `cc2530/hal/clock.h`
- `cc2530/hal/hal_assert.c` 和 `cc2530/hal/hal_assert.h`
- `cc2530/hal/hal_defs.h`
- `cc2530/hal/hal_int.c` 和 `cc2530/hal/hal_int.h`
- `cc2530/hal/hal_mcu.c` 和 `cc2530/hal/hal_mcu.h`
- `cc2530/hal/hal_rf.c` 和 `cc2530/hal/hal_rf.h`
- `cc2530/hal/hal_types.h`

TI 条款将这些代码的使用、修改、复制和分发限制为：嵌入 Texas Instruments 微控制器的软件，或仅与 Texas Instruments 射频收发器配合使用的软件。这些文件继续受其原有条款约束，本项目不对其重新授权。`clock.h` 标明了 TI CC253x 来源，但没有附带完整许可声明，因此在此单独列出。
