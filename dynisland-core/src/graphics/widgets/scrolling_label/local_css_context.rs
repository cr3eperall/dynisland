use gtk::CssProvider;
use rand::{distributions::Alphanumeric, Rng};

use crate::{
    config_variable::ConfigVariable, graphics::util::CssSize, implement_config_get_set
};

#[derive(Clone, glib::Boxed, Debug)]
#[boxed_type(name = "BoxedScrollingLabelLocalCssContext")]
pub struct ScrollingLabelLocalCssContext {
    //IMPORTANT add some way to globally configure and swap the animations (maybe get a string to format from a config)
    css_provider: CssProvider,
    name: String,
    size: i32,
    anim_restart_flag: bool,
    active: bool,

    config_fade_size: ConfigVariable<CssSize>,
    config_speed: ConfigVariable<f32>, //pixels per second
    config_delay: ConfigVariable<u64>, //millis
}

#[allow(unused_braces)]
impl ScrollingLabelLocalCssContext {
    pub fn new(name: &str) -> Self {
        Self {
            css_provider: gtk::CssProvider::new(),
            name: name.to_string(),
            anim_restart_flag: false,
            size: 0,
            active: true,
            config_fade_size: ConfigVariable::new(CssSize::Percent(4.0)),
            config_speed: ConfigVariable::new(40.0),
            config_delay: ConfigVariable::new(2000),
        }
    }

    implement_config_get_set!(pub, config_fade_size, CssSize);
    implement_config_get_set!(pub, config_speed, f32);
    implement_config_get_set!(pub, config_delay, u64);

    // GET
    pub fn get_css_provider(&self) -> CssProvider {
        self.css_provider.clone()
    }
    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn get_size(&self) -> i32 {
        self.size
    }
    pub fn get_active(&self) -> bool {
        self.active
    }

    // SET
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
        self.update_provider()
    }
    pub fn set_active(&mut self, active: bool, size: i32) {
        if self.active == active && self.size == size {
            return;
        }
        if active && self.size != size {
            self.anim_restart_flag = !self.anim_restart_flag;
        }
        self.active = active;
        self.size = size;
        self.update_provider()
    }

    fn update_provider(&self) {
        // let border_radius = self.border_radius;
        let name = self.name.as_str();
        let active = self.active;
        let size = self.size;
        let mut duration = (size as f32 / self.config_speed.value) * 1000.0; //millis
        let delay = self.config_delay.value as f32;
        duration += delay;
        let start_percentage = (delay / duration) * 100.0;

        // log::debug!("size: {size}");
        // debug!("{size_timing_function}");
        let scroll_anim = if self.anim_restart_flag {
            "scroll-clone"
        } else {
            "scroll"
        };
        let css = if active {
            // log::debug!("active");
            //TODO add duration, delay, timing function to config
            format!(
                r"@keyframes scroll {{ /* need 2 animations to swap when i want to reset it */
                    0%    {{ transform: translateX(0px); }}
                    {start_percentage:.3}% {{ transform: translateX(0px); }}
                    100%  {{ transform: translateX(-{size}px); }}
                }}
                @keyframes scroll-clone {{
                    0%    {{ transform: translateX(0px); }}
                    {start_percentage:.3}% {{ transform: translateX(0px); }}
                    100%  {{ transform: translateX(-{size}px); }}
                }}
                .{name} > box {{
                    animation: none;
                    transform: translateX(0px);
                    animation-name: {scroll_anim};
                    animation-duration: {duration}ms;
                    animation-iteration-count: infinite;
                    animation-timing-function: linear;
                    /* animation-delay: 1s; */
                }}"
            )
        } else {
            format!(
                r".{name}> box{{ 
                    animation: none;
                }}"
            )
        };
        // log::debug!("{css}");
        self.css_provider.load_from_string(&css);
    }
}

impl Default for ScrollingLabelLocalCssContext {
    fn default() -> Self {
        Self::new(
            "c".chars()
                .chain(
                    rand::thread_rng()
                        .sample_iter(&Alphanumeric)
                        .take(6)
                        .map(char::from),
                )
                .collect::<String>()
                .as_str(),
        )
    }
}
