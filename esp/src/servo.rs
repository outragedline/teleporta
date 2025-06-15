use esp_idf_svc::hal::ledc::LedcDriver;

pub struct Servo<'a> {
    pub channel_driver: LedcDriver<'a>,
}
impl Servo<'_> {
    pub fn close(&mut self) {
        let max_duty = self.channel_driver.get_max_duty();
        let angle_0 = (max_duty as f32 * 0.025) as u32;
        self.channel_driver.set_duty(angle_0).unwrap();
    }

    pub fn open(&mut self) {
        let max_duty = self.channel_driver.get_max_duty();
        self.close();
        let angle_90 = (max_duty as f32 * 0.085) as u32;
        self.channel_driver.set_duty(angle_90).unwrap();
    }
}
