#ifndef CC2530_UART_H
#define CC2530_UART_H

#include "platform.h"

#define UART0_RX_EMPTY 0U
#define UART0_RX_BYTE 1U
#define UART0_RX_OVERFLOW 2U

/*
 * UART0 位置 1：P0.2 RX、P0.3 TX，115200 8N1，无流控。
 * 初始化切换到 32 MHz 晶振，清空接收队列，开启 UART0 接收和总中断。
 * 本模块独占 UART0 及其接收中断。时钟改变后须重新初始化。
 */
void uart0_init(void);

/*
 * 主循环中非阻塞读取，byte 必须指向可写的单字节。
 * 返回 UART0_RX_BYTE 时写入 byte（包括 NUL）；EMPTY 时不修改 byte。
 * OVERFLOW 表示有数据丢失，驱动已清空积压队列；应用负责协议重新同步。
 * 队列可容纳 127 字节。本函数保留调用前的接收中断使能状态。
 */
uint8 uart0_read(uint8 *byte);

/* 主循环中阻塞发送；发送期间不关闭接收，不可从中断中调用。 */
void uart0_send_byte(uint8 byte);
/* 发送 NUL 结尾字符串，不发送终止 NUL，不自动追加换行。 */
void uart0_send_text(const char *text);

/* SDCC 要求入口文件可见中断声明，才能为独立编译的 uart.c 生成向量。 */
PLATFORM_ISR(uart0_rx_isr, URX0_VECTOR);

#endif
