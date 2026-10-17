#include "uart.h"
#include "hal_defs.h"

#define LED1_MASK BV(0)
#define LED2_MASK BV(1)
#define LED3_MASK BV(4)
#define LED_MASK (LED1_MASK | LED2_MASK | LED3_MASK)
#define COMMAND_SIZE 2U

static uint8 command[COMMAND_SIZE];
static uint8 command_length;
static uint8 discard_until_end;

static void execute_command(void)
{
    uint8 mask;

    if (command_length != COMMAND_SIZE ||
        (command[1] != '0' && command[1] != '1')) {
        uart0_send_text("error:command\r\n");
        return;
    }

    switch (command[0]) {
    case 'R': mask = LED1_MASK; break;
    case 'G': mask = LED2_MASK; break;
    case 'Y': mask = LED3_MASK; break;
    case 'A': mask = LED_MASK; break;
    default:
        uart0_send_text("error:command\r\n");
        return;
    }

    if (command[1] == '1') {
        P1 &= (uint8)~mask;
    } else {
        P1 |= mask;
    }

    uart0_send_text("send:");
    uart0_send_byte(command[0]);
    uart0_send_byte(command[1]);
    uart0_send_text("\r\n");
}

static void receive_byte(uint8 byte)
{
    if (discard_until_end) {
        if (byte == '#') {
            discard_until_end = 0;
        }
    } else if (byte == '#') {
        execute_command();
        command_length = 0;
    } else if (command_length == 0 && (byte == '\r' || byte == '\n')) {
        /* 兼容串口助手在完整命令后自动追加的换行。 */
    } else if (command_length < COMMAND_SIZE) {
        command[command_length++] = byte;
    } else {
        /* 超长命令整帧作废，不能把尾部误当成下一条有效命令。 */
        command_length = 0;
        discard_until_end = 1;
        uart0_send_text("error:command\r\n");
    }
}

PLATFORM_MAIN
{
    uint8 byte;
    uint8 result;

    P1SEL &= (uint8)~LED_MASK;
    P1 |= LED_MASK;               /* 上电时全部熄灭。 */
    P1DIR |= LED_MASK;
    uart0_init();
    uart0_send_text("UART0 LED ready\r\n");

    while (1) {
        result = uart0_read(&byte);
        if (result == UART0_RX_OVERFLOW) {
            command_length = 0;
            discard_until_end = 1;
            uart0_send_text("error:rx overflow\r\n");
        } else if (result == UART0_RX_BYTE) {
            receive_byte(byte);
        }
    }
}
