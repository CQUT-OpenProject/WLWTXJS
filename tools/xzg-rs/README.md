# xzg-rs

`xzg-rs` 是用 Rust 编写的 CC2530 烧录工具，可通过 TI CC Debugger 或兼容的 SmartRF04EB 连接开发板。

## 编译

在仓库根目录运行：

```sh
cargo build --release --manifest-path tools/xzg-rs/Cargo.toml
```

生成的可执行文件位于 `tools/xzg-rs/target/release/xzg`。

## 命令

检查固件文件，并探测已连接的目标芯片：

```sh
tools/xzg-rs/target/release/xzg check-hex path/to/firmware.hex
tools/xzg-rs/target/release/xzg probe-ti
```

`check-hex` 检查 Intel HEX 固件文件。`probe-ti` 会报告烧录器、芯片 ID、调试器固件版本和芯片出厂 IEEE 地址。请核对探测结果，确认目标设备是准备烧录的开发板。

烧录固件时，工具默认依次执行芯片擦除、写入、回读校验和复位：

```sh
tools/xzg-rs/target/release/xzg flash-ti path/to/firmware.hex
```

可以使用 `--no-erase`、`--no-write` 或 `--no-verify` 跳过相应步骤。工具会把 HEX 数据间隙和镜像末尾填为 `0xFF`；启用写入或校验时，这些填充字节也会参与相应步骤。若跳过擦除，目标对应范围应已擦除，否则 Flash 无法把已清零的位改回 1，回读校验会失败。

`check-hex` 和 `probe-ti` 的结果以 JSON 输出；进度信息和错误信息写入标准错误输出（stderr）。

项目来源与许可信息见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
