use embassy_rp::Peri;
use embassy_rp::clocks::clk_sys_freq;
use embassy_rp::pwm::{self, ChannelAPin, Config, Pwm};
use fixed::FixedU16;
use fixed::types::extra::U4;

pub struct Sg90<'d> {
    pwm: Pwm<'d>,
    config: Config,
}

impl<'d> Sg90<'d> {
    pub fn new<T: pwm::Slice, P: ChannelAPin<T>>(slice: Peri<'d, T>, pin: Peri<'d, P>) -> Self {
        let mut config = Config::default();

        let sys_freq = clk_sys_freq();
        let divider = FixedU16::<U4>::from_num(sys_freq) / FixedU16::<U4>::from_num(1_000_000u32);

        config.divider = divider;

        config.top = 19_999;

        config.compare_a = 1000;

        Self {
            pwm: Pwm::new_output_a(slice, pin, config.clone()),
            config,
        }
    }

    pub fn set_angle(&mut self, angle: f32) {
        let clamped_angle = angle.clamp(0.0, 180.0);

        let pulse_width_us = 1000.0 + (clamped_angle / 180.0) * 1000.0;

        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "Already clamped into an acceptable range"
        )]
        {
            self.config.compare_a = pulse_width_us as u16;
        }

        self.pwm.set_config(&self.config);
    }

    #[expect(unused, reason = "not used yet")]
    pub fn set_pulse_us(&mut self, pulse_us: u16) {
        let clamped_pulse = pulse_us.clamp(1000, 2000);

        self.config.compare_a = clamped_pulse;
        self.pwm.set_config(&self.config);
    }
}
