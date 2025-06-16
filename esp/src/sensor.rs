use esp_idf_hal::gpio::{AnyInputPin, AnyOutputPin};
use esp_idf_svc::{
    hal::{
        delay::Ets,
        gpio::{Input, Output, PinDriver},
    },
    sys::esp_timer_get_time,
};

const SENSOR_TIMEOUT_US: i64 = 30000;

pub struct HcSr04<'d> {
    trig: PinDriver<'d, AnyOutputPin, Output>,
    echo: PinDriver<'d, AnyInputPin, Input>,
}

impl<'d> HcSr04<'d> {
    pub fn new(
        trig_pin: impl Into<AnyOutputPin>,
        echo_pin: impl Into<AnyInputPin>,
    ) -> anyhow::Result<Self> {
        let trig = PinDriver::output(trig_pin.into())?;
        let echo = PinDriver::input(echo_pin.into())?;
        Ok(Self { trig, echo })
    }

    /// Done by ai validated by me because theres no way i would know to do it
    /// Measures the distance in centimeters.
    pub fn measure_distance_cm(&mut self) -> anyhow::Result<f32> {
        self.trig.set_low()?;
        Ets::delay_us(5);
        self.trig.set_high()?;
        Ets::delay_us(10);
        self.trig.set_low()?;

        let start_time_wait = unsafe { esp_timer_get_time() };
        while self.echo.is_low() {
            if unsafe { esp_timer_get_time() } - start_time_wait > SENSOR_TIMEOUT_US {
                return Err(anyhow::anyhow!("Timeout waiting for echo pulse to start"));
            }
        }

        // 3. Measure the duration of the echo pulse
        let pulse_start_time = unsafe { esp_timer_get_time() };
        while self.echo.is_high() {
            if unsafe { esp_timer_get_time() } - pulse_start_time > SENSOR_TIMEOUT_US {
                return Err(anyhow::anyhow!("Timeout waiting for echo pulse to end"));
            }
        }
        let pulse_end_time = unsafe { esp_timer_get_time() };

        let pulse_duration_us = pulse_end_time - pulse_start_time;

        // 4. Calculate the distance
        // Distance (m) = (Duration (s) * Speed of Sound (m/s)) / 2
        // We convert everything to use microseconds and centimeters for better precision with integers.
        // Distance (cm) = (Duration (µs) * Speed of Sound (cm/µs)) / 2
        // Speed of Sound = 343 m/s = 0.0343 cm/µs
        let distance_cm = (pulse_duration_us as f32 * 0.0343) / 2.0;

        Ok(distance_cm)
    }
}
