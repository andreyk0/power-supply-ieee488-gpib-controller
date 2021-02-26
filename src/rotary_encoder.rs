//! Sets up timer to capture input from a rotary encoder

use stm32f1xx_hal::pac::{RCC, TIM1};

pub struct RotaryEncoder {
    timer: TIM1,
    pub count: i16,
}

impl RotaryEncoder {
    pub fn new(mut timer: TIM1) -> Self {
        setup_rotary_encoder_timer(&mut timer);

        RotaryEncoder { timer, count: 0 }
    }

    pub fn poll(&mut self) -> i16 {
        let cnt = (self.timer.cnt.read().bits() as i16) >> 2;
        let (diff, _) = cnt.overflowing_sub(self.count);
        self.count = cnt;
        diff
    }
}

/// Use timer as a rotary encoder.
///
/// Example from the docs:
/// TIM configuration in encoder mode
///   1. Select and configure the timer input:
///   • Input selection:
///   •
///   – TI1 connected to TI1FP1 CC1S='01' in CCMR1 register
///   – TI2 connected to TI2FP2 CC2S='01' in CCMR1 register
///   Input polarity:
///   – CC1P='0' and CC1NP='0'(CCER register, TI1FP1 non-inverted, TI1FP1=TI1).
///   – CC2P='0' and CC2NP='0'(CCER register, TI1FP2 non-inverted, TI1FP2= TI2).
///   2. Select the encoder mode
///   • Encoder mode1 (resolution X2 on TI2): SMS=’001’ in SMCR register.
///   • Encoder mode2 (resolution X2 on TI1): SMS=’010' in SMCR register.
///   • Encoder mode3 (resolution X4 on TI1 and TI2): SMS=’011’ in SMCR register.
///   3. Enable the timer counter
///   • Set the counter enable bit, CEN='1' in CR1 register.
///
///
/// IC1F[3:0]: Input capture 1 filter
/// This bit-field defines the frequency used to sample TI1 input and the length of the digital filter applied
/// to TI1. The digital filter is made of an event counter in which N consecutive events are needed to
/// validate a transition on the output:
/// 0000: No filter, sampling is done at f DTS
/// 0001: f SAMPLING =f CK_INT , N=2
/// 0010: f SAMPLING =f CK_INT , N=4
/// 0011: f SAMPLING =f CK_INT , N=8
/// 0100: f SAMPLING =f DTS /2, N=6
/// 0101: f SAMPLING =f DTS /2, N=8
/// 0110: f SAMPLING =f DTS /4, N=6
/// 0111: f SAMPLING =f DTS /4, N=8
/// 1000: f SAMPLING =f DTS /8, N=6
/// 1001: f SAMPLING =f DTS /8, N=8
/// 1010: f SAMPLING =f DTS /16, N=5
/// 1011: f SAMPLING =f DTS /16, N=6
/// 1100: f SAMPLING =f DTS /16, N=8
/// 1101: f SAMPLING =f DTS /32, N=5
/// 1110: f SAMPLING =f DTS /32, N=6
/// 1111: f SAMPLING =f DTS /32, N=8
///
fn setup_rotary_encoder_timer(tim: &mut TIM1) {
    // NOTE(unsafe) This executes only during initialisation
    let rcc = unsafe { &(*RCC::ptr()) };

    rcc.apb2enr.modify(|_, w| w.tim1en().set_bit()); // enable clock

    tim.ccmr1_input_mut().modify(|_, w| {
        w.cc1s()
            .ti1() // 01: CC1 channel is configured as input, IC1 is mapped on TI1
            .cc2s()
            .ti2() // 01: CC2 channel is configured as input, IC2 is mapped on TI2
            .ic1f()
            .bits(0b1111) // input capture 1 filter
            .ic2f()
            .bits(0b1111) // input capture 2 filter
    });
    tim.ccer.modify(|_, w| {
        // CC1NP/CC1P bits select the active polarity of TI1FP1 and TI2FP1 for trigger or capture operations.
        // 01: inverted/falling edge
        //   The circuit is sensitive to TIxFP1 falling edge (capture or trigger operations in reset, external
        //    clock or trigger mode), TIxFP1 is inverted (trigger operation in gated mode or encoder mode)
        w.cc1p()
            .set_bit() // active low
            .cc1np()
            .clear_bit()
            .cc2p()
            .set_bit() // active low
            .cc2np()
            .clear_bit()
    });
    tim.smcr.modify(|_, w| {
        w.sms().bits(0b011) // Encoder mode3 (resolution X4 on TI1 and TI2): SMS=’011’ in SMCR register.
    });

    tim.cr1.modify(|_, w| w.cen().set_bit()); // enable counter
}
