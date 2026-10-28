/*
 * 实验 4：协议栈工作流程和无线收发控制 LED
 *
 * 原实验使用 IAR + Z-Stack，协调器与终端组网后通过周期消息 "D1" 互相控制
 * D1 闪烁。本文件使用 BasicRF 替代 Z-Stack：BasicRF 不提供组网/入网流程，
 * 两个节点以相同的 PAN ID 和信道直接点对点通信。
 *
 * 协调器（ROLE_COORDINATOR）：
 *   - 上电初始化完成后 D1 闪烁 3 次，表示射频已就绪；
 *   - 收到 "D1" 后闪烁 D1，并把 "D1" 回发给终端。
 *
 * 终端（ROLE_ENDDEVICE）：
 *   - 上电初始化完成后 D1 闪烁 3 次；
 *   - 延时 1 秒后每隔 1 秒发送一次 "D1"；
 *   - 收到协调器回发的 "D1" 后闪烁 D1。
 *
 * 注意：BasicRF 的接收路径由 basicRfPoll() 在主循环中轮询完成，本程序不开启
 * 全局中断（EA），因此不要调用 uart0_init() 等会置位 EA 的函数。
 */

#include "clock.h"
#include "hal_mcu.h"
#include "basic_rf.h"
#include "platform.h"

#define LED1_MASK          BV(0)      /* D1 接在 P1.0，低电平点亮。 */
#define RF_CHANNEL         11U
#define PAN_ID             0x2007U

#define BLINK_PHASE_MS     120U
#define STARTUP_BLINKS     3U
#define START_DELAY_MS     1000U
#define SEND_PERIOD_MS     1000U

#if defined(ROLE_COORDINATOR)
#define NODE_ADDR          0x0000U
#define PEER_ADDR          0x0001U
#define IS_COORDINATOR     1
#elif defined(ROLE_ENDDEVICE)
#define NODE_ADDR          0x0001U
#define PEER_ADDR          0x0000U
#define IS_COORDINATOR     0
#else
#error "Define ROLE_COORDINATOR or ROLE_ENDDEVICE when compiling"
#endif

static basicRfCfg_t rf_config = {
    NODE_ADDR,
    PAN_ID,
    RF_CHANNEL,
    FALSE               /* 不使用硬件 ACK，依靠周期性发送容忍丢包。 */
};

static uint8 payload[2] = { 'D', '1' };

static uint16 last_timer_count;
static uint32 timer_fraction;
static uint32 elapsed_ms;

static uint8 led_on;
static uint8 blink_phases;
static uint32 blink_deadline;

#if !IS_COORDINATOR
static uint32 next_send_ms;
#endif

/* Timer 1：128 分频、自由运行，计数器每 4 us 加 1。 */
static uint16 timer_read(void)
{
    uint8 low = T1CNTL;
    uint8 high = T1CNTH;

    return ((uint16)high << 8) | low;
}

static void timer_init(void)
{
    T1CTL = 0x00;               /* 先停止计数器。 */
    T1CNTL = 0x00;              /* 写低字节会把 16 位计数器清零。 */
    T1CNTH = 0x00;
    T1CTL = 0x0D;              /* DIV=11（128 分频），MODE=01（自由运行）。 */
    last_timer_count = timer_read();
    timer_fraction = 0;
    elapsed_ms = 0;
}

static uint32 millis(void)
{
    uint16 now = timer_read();
    uint16 delta = (uint16)(now - last_timer_count);

    last_timer_count = now;
    timer_fraction += delta;
    while (timer_fraction >= 250U) { /* 250 kHz -> 250 个计数为 1 ms。 */
        timer_fraction -= 250U;
        ++elapsed_ms;
    }

    return elapsed_ms;
}

static void led_set(uint8 on)
{
    if (on) {
        P1 &= (uint8)~LED1_MASK;
    } else {
        P1 |= LED1_MASK;
    }
    led_on = on;
}

/*
 * 请求闪烁若干次。blink_phases 记录剩余的“亮/灭”半周期数：
 * 每次请求 1 次闪烁对应 2 个半周期，先点亮，再熄灭。
 */
static void blink_request(uint8 flashes, uint32 now)
{
    uint8 phases = (uint8)(flashes * 2U);

    if (phases > blink_phases) {
        blink_phases = phases;
    }
    if (blink_phases != 0U && !led_on) {
        led_set(TRUE);
        blink_deadline = now + BLINK_PHASE_MS;
    }
}

static void blink_poll(uint32 now)
{
    if (blink_phases == 0U) {
        return;
    }
    if ((int32)(now - blink_deadline) < 0) {
        return;
    }

    --blink_phases;
    if (blink_phases == 0U) {
        led_set(FALSE);
    } else {
        led_set((uint8)!led_on);
        blink_deadline = now + BLINK_PHASE_MS;
    }
}

PLATFORM_MAIN
{
    uint8 buffer[4];
    uint8 length;
    int16 rssi;
    uint32 now;

    P1SEL &= (uint8)~LED1_MASK;
    P1DIR |= LED1_MASK;
    led_set(FALSE);

    halMcuInit();
    timer_init();

    if (basicRfInit(&rf_config) == FAILED) {
        /* 射频初始化失败：D1 持续快速闪烁，便于和正常运行区分。 */
        while (1) {
            led_set(TRUE);
            halMcuWaitMs(100);
            led_set(FALSE);
            halMcuWaitMs(100);
        }
    }
    /*
     * 本 SDCC 移植中硬件帧过滤会导致所有包都被丢弃（Breezio-neo 亦记录为
     * 实测问题），因此关闭 FRMFILT0 的 FRAME_FILTER_EN。本实验场景中只有
     * 一个终端在发送 "D1"，应用层按负载内容过滤即可。
     */
    FRMFILT0 = 0x00;
    basicRfReceiveOn();

    now = millis();
    blink_request(STARTUP_BLINKS, now);
#if !IS_COORDINATOR
    next_send_ms = now + START_DELAY_MS;
#endif

    while (1) {
        basicRfPoll();

        if (basicRfPacketIsReady()) {
            length = basicRfReceive(buffer, (uint8)sizeof(buffer), &rssi);
            if (length >= 2U && buffer[0] == 'D' && buffer[1] == '1') {
                now = millis();
                blink_request(1U, now);
#if IS_COORDINATOR
                /* 协调器收到 "D1" 后原样回发，终端收到后闪烁。 */
                basicRfSendPacket(PEER_ADDR, payload, (uint8)sizeof(payload));
#endif
            }
        }

        now = millis();
        blink_poll(now);

#if !IS_COORDINATOR
        if ((int32)(now - next_send_ms) >= 0) {
            basicRfSendPacket(PEER_ADDR, payload, (uint8)sizeof(payload));
            next_send_ms = now + SEND_PERIOD_MS;
        }
#endif
    }
}
