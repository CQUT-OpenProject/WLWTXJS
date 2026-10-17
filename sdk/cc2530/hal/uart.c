#include "uart.h"
#include "clock.h"

#define UART0_PINS (BV(2) | BV(3))
#define RX_RING_SIZE 128U
#define RX_RING_MASK (RX_RING_SIZE - 1U)

static volatile PLATFORM_XDATA uint8 rx_ring[RX_RING_SIZE];
static volatile uint8 rx_head;
static volatile uint8 rx_tail;
static volatile uint8 rx_overflow;

void uart0_init(void)
{
    URX0IE = 0;
    U0CSR &= (uint8)~0x40;
    rx_head = 0;
    rx_tail = 0;
    rx_overflow = 0;
    clockSetMainSrc(CLOCK_SRC_XOSC);

    PERCFG &= (uint8)~BV(0);       /* USART0 位置 1：P0.2 RX、P0.3 TX。 */
    P0SEL |= UART0_PINS;           /* 不占用未启用流控的 P0.4、P0.5。 */
    P2DIR &= (uint8)~0xC0;         /* P0 上 USART0 优先。 */
    U0CSR = 0x80;                 /* UART 模式，配置期间暂不接收。 */
    U0UCR = 0x02;                 /* 8 位数据、无校验、1 个停止位，无流控。 */
    U0GCR = 11;                   /* 32 MHz：BAUD_E=11、BAUD_M=216。 */
    U0BAUD = 216;                 /* 实际波特率约 115234 bps。 */
    UTX0IF = 0;
    URX0IF = 0;
    U0CSR |= 0x40;                /* 开启接收器。 */
    URX0IE = 1;
    EA = 1;
}

PLATFORM_ISR(uart0_rx_isr, URX0_VECTOR)
{
    uint8 byte;
    uint8 next_head;

    URX0IF = 0;
    byte = U0DBUF;
    if (rx_overflow) {
        return;                   /* 丢失数据后等待主循环重新同步。 */
    }

    next_head = (uint8)((rx_head + 1U) & RX_RING_MASK);
    if (next_head == rx_tail) {
        rx_overflow = 1;           /* 留一个空位区分队列满和空。 */
    } else {
        rx_ring[rx_head] = byte;
        rx_head = next_head;
    }
}

/* 返回 0 表示无数据，1 表示取到字节，2 表示接收队列曾溢出。 */
uint8 uart0_read(uint8 *byte)
{
    uint8 result = UART0_RX_EMPTY;
    uint8 rx_interrupt_enabled = URX0IE;

    URX0IE = 0;
    if (rx_overflow) {
        rx_tail = rx_head;
        rx_overflow = 0;
        result = UART0_RX_OVERFLOW;
    } else if (rx_tail != rx_head) {
        *byte = rx_ring[rx_tail];
        rx_tail = (uint8)((rx_tail + 1U) & RX_RING_MASK);
        result = UART0_RX_BYTE;
    }
    URX0IE = rx_interrupt_enabled;
    return result;
}

void uart0_send_byte(uint8 byte)
{
    UTX0IF = 0;                   /* 每次写入前清零，避免误用上次完成标志。 */
    U0DBUF = byte;
    while (!UTX0IF) {
    }
}

void uart0_send_text(const char *text)
{
    while (*text) {
        uart0_send_byte((uint8)*text++);
    }
}

