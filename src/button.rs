//! Push button with debounce

use rtic::cyccnt::Instant;

use embedded_hal::digital::v2::InputPin;

use stm32f1xx_hal::{
    afio,
    gpio::{Edge, ExtiPin},
    pac::EXTI,
    time::MilliSeconds,
};

use crate::prelude::*;

pub struct Button<Pin> {
    last_press: Instant,
    last_change: Instant,
    last_state: bool,
    last_push_duration_millis: Option<MilliSeconds>,
    pin: Pin,
}

impl<Pin> Button<Pin>
where
    Pin: InputPin + ExtiPin,
{
    pub fn new(mut pin: Pin, exti: &EXTI, afio: &mut afio::Parts) -> Result<Self, AppError> {
        pin.make_interrupt_source(afio);
        pin.trigger_on_edge(exti, Edge::RISING_FALLING);
        pin.enable_interrupt(exti);

        Ok(Button {
            last_press: Instant::now(),
            last_change: Instant::now(),
            last_state: pin.is_high().map_err(|_| AppError::Duh)?, // should be Infallible
            last_push_duration_millis: None,
            pin,
        })
    }

    pub fn is_pressed(&self, min_press_duration: MilliSeconds) -> bool {
        let now = Instant::now();
        let cd_millis = duration_since_millis(now, self.last_change);
        self.last_state && (cd_millis > min_press_duration)
    }

    pub fn take_last_press(&mut self, min_press_duration: MilliSeconds) -> Option<MilliSeconds> {
        self.last_push_duration_millis
            .take()
            .filter(|d| d.0 >= min_press_duration.0)
    }

    pub fn poll(&mut self) -> Result<(), AppError> {
        let current_state = self.is_high()?;
        let now = Instant::now();

        if self.last_state != current_state {
            if current_state {
                let pd_millis = duration_since_millis(now, self.last_press);

                match self.last_push_duration_millis {
                    None => self.last_push_duration_millis = Some(pd_millis),
                    Some(pd0) => {
                        // prefer longer press in case an even was missed
                        self.last_push_duration_millis = Some(MilliSeconds(pd_millis.0.max(pd0.0)))
                    }
                }
            } else {
                self.last_press = now;
            }

            self.last_change = now;
        }

        self.last_state = current_state;
        self.pin.clear_interrupt_pending_bit();
        Ok(())
    }

    #[inline]
    fn is_high(&mut self) -> Result<bool, AppError> {
        self.pin.is_high().map_err(|_| AppError::Duh) // should be Infallible
    }
}

#[inline]
fn duration_since_millis(newer: Instant, older: Instant) -> MilliSeconds {
    MilliSeconds(if newer > older {
        let ticks = newer.duration_since(older);
        ticks.as_cycles() / SYS_CYCLES_PER_MILLISECOND
    } else {
        0 // overflow
    })
}
