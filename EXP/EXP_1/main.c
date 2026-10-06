#include "platform.h"

#define LED1_MASK 0x01
#define LED2_MASK 0x02
#define LED3_MASK 0x10
#define LED_MASK (LED1_MASK | LED2_MASK | LED3_MASK)

static void delay(void)
{
    volatile uint16 i;

    for (i = 0; i < 10000; ++i) {
    }
    for (i = 0; i < 10000; ++i) {
    }
    for (i = 0; i < 10000; ++i) {
    }
    for (i = 0; i < 10000; ++i) {
    }
    for (i = 0; i < 10000; ++i) {
    }
}

static void show_led(uint8 mask)
{
    P1 |= LED_MASK;             /* 先熄灭所有 LED。 */
    P1 &= (uint8)~mask;         /* 低电平点亮当前 LED。 */
}

PLATFORM_MAIN
{
    P1SEL &= (uint8)~LED_MASK;
    P1DIR |= LED_MASK;
    P1 |= LED_MASK;             /* 板载 LED 低电平点亮，初始化为熄灭。 */

    while (1) {
        show_led(LED1_MASK);
        delay();
        show_led(LED2_MASK);
        delay();
        show_led(LED3_MASK);
        delay();
        show_led(LED2_MASK);
        delay();
    }
}
