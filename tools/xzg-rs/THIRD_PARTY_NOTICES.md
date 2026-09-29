# 第三方声明

## Breezio-neo `xzg_cli`

本工具参考了 `martin-cao/Breezio-neo`，并用 Rust 重写了其中的 USB 烧录流程、TI CC Debugger 协议实现和 Intel HEX 处理逻辑。Breezio-neo 使用 Apache License 2.0 许可。项目保留了原项目及版权声明，详见 [`LICENSE`](LICENSE)。`src/` 中的 Rust 源码是重写版本，不是从原 Python 源码复制而来。

## XZG-MT

CC Debugger 协议研究和烧录流程参考了 XZG-MT。Breezio-neo 的上游声明指出，相关 XZG-MT 内容采用 MIT 许可，版权归 xyzroe 所有（2025 年）。MIT 许可全文如下：

MIT License

Copyright (c) 2025 xyzroe

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
