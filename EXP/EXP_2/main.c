#include "clock.h"

#define LED1_MASK BV(0)
#define LED2_MASK BV(1)
#define LED3_MASK BV(4)
#define LED_MASK (LED1_MASK | LED2_MASK | LED3_MASK)
#define KEY1_MASK BV(1)

#define TIMER1_PRESCALER 128UL
#define TIMER1_DIV_128 (3U << 2) /* T1CTL.DIV[1:0] = 11：128 分频。 */
#define TIMER1_MODE_MODULO 2U
#define TIMER1_TICK_HZ (32000000UL / TIMER1_PRESCALER)
#define TIMER1_PERIOD_TICKS (TIMER1_TICK_HZ / 1000UL)
#define LED_STEP_MS 250U
#define KEY_DEBOUNCE_MS 20U
#define SHUTDOWN_DELAY_MS 5000U

static volatile uint16 led_elapsed_ms;
static volatile uint16 key_elapsed_ms;
static volatile uint16 shutdown_elapsed_ms;
static volatile uint8 led_step;
static volatile uint8 key_down;
static volatile uint8 shutdown_requested;
static volatile uint8 shutdown_complete;

static void show_led(uint8 mask)
{
    P1 |= LED_MASK;             /* 先熄灭所有 LED。 */
    P1 &= (uint8)~mask;         /* 低电平点亮当前 LED。 */
}

static void show_next_led(void)
{
    led_step = (uint8)((led_step + 1) & 0x03);
    switch (led_step) {
    case 0:
        show_led(LED1_MASK);
        break;
    case 1:
        show_led(LED2_MASK);
        break;
    case 2:
        show_led(LED3_MASK);
        break;
    default:
        show_led(LED2_MASK);
        break;
    }
}

PLATFORM_ISR(timer1_isr, T1_VECTOR)
{
    /* 本中断由 Timer 1 通道 0 的比较事件触发；先清标志，等待下一次比较事件。 */
    T1STAT = (uint8)~BV(0);

    /* 关灯流程已经完成时，不再执行按键和 LED 计时逻辑。 */
    if (shutdown_complete) {
        return;
    }

    /* 按键已确认按下：每次约 1 ms 的中断累计一次，等待 5 秒后关灯并停表。 */
    if (shutdown_requested) {
        ++shutdown_elapsed_ms;
        if (shutdown_elapsed_ms >= SHUTDOWN_DELAY_MS) {
            T1CTL = 0x00;                   /* 停止 Timer 1。 */
            T1CCTL0 = 0x00;                  /* 关闭通道 0 的比较功能及中断。 */
            IEN1 &= (uint8)~BV(1);           /* 禁止 Timer 1 中断。 */
            show_led(0);                     /* 掩码为 0，不点亮任何 LED。 */
            shutdown_complete = 1;           /* 标记流程完成，后续中断直接返回。 */
            return;
        }
    } else if ((P0 & KEY1_MASK) == 0) {
        /* 尚未启动关灯倒计时；检测到 P0.1 为低电平，表示 KEY1 按下。 */
        if (!key_down) {
            key_down = 1;                    /* 记录首次检测到按下。 */
            key_elapsed_ms = 1;              /* 开始累计连续按下的时间。 */
        } else if (key_elapsed_ms < KEY_DEBOUNCE_MS) {
            ++key_elapsed_ms;                /* 消抖期间持续低电平才累计时间。 */
        }

        if (key_elapsed_ms >= KEY_DEBOUNCE_MS) {
            shutdown_requested = 1;          /* 持续按下达到消抖时间，确认按键有效。 */
            shutdown_elapsed_ms = 0;          /* 从此刻开始计算 5 秒关灯延时。 */
        }
    } else {
        key_down = 0;                         /* 按键已松开，清除按下状态。 */
        key_elapsed_ms = 0;                   /* 按下时间归零，重新进行消抖判断。 */
    }

    /* 按节拍累计 LED 点亮时间；倒计时期间流水灯仍继续运行。 */
    ++led_elapsed_ms;
    if (led_elapsed_ms >= LED_STEP_MS) {
        led_elapsed_ms = 0;                  /* 到达一个流水步长，开始下一轮计时。 */
        show_next_led();                     /* 按 LED1、LED2、LED3、LED2 的顺序切换。 */
    }
}

static void start_timer1(void)
{
    T1CTL = 0x00;               /* 配置期间暂停 Timer 1。 */
    T1CNTL = 0x00;              /* 写低字节会将 16 位计数器清零。 */
    T1CC0L = (uint8)(TIMER1_PERIOD_TICKS - 1U);
    T1CC0H = 0x00;
    T1STAT = 0x00;              /* 清除可能遗留的定时器标志。 */
    TIMIF &= (uint8)~BV(6);      /* 屏蔽模计数终点标志，只使用通道比较中断。 */
    T1CCTL0 = 0x44;             /* 通道 0 比较模式，并启用比较中断。 */
    IEN1 |= BV(1);              /* 允许 Timer 1 中断。 */
    IEN0 |= BV(7);              /* 开启总中断。 */
    T1CTL = TIMER1_DIV_128 | TIMER1_MODE_MODULO; /* 0x0E，128 分频、模计数。 */
}

PLATFORM_MAIN
{
    P1SEL &= (uint8)~LED_MASK;
    P1DIR |= LED_MASK;
    P1 |= LED_MASK;             /* 板载 LED 低电平点亮，初始化为熄灭。 */

    P0SEL &= (uint8)~KEY1_MASK;
    P0DIR &= (uint8)~KEY1_MASK; /* KEY1 接在 P0.1，低电平按下。 */

    clockSetMainSrc(CLOCK_SRC_XOSC);
    show_led(LED1_MASK);
    start_timer1();

    while (1) {
    }
}
