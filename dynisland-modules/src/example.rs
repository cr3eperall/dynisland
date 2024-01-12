use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    str::FromStr,
    sync::Arc,
    vec,
};

use anyhow::{Context, Ok, Result};
use async_trait::async_trait;
use css_anim::soy::Bezier;
use gtk::prelude::*;
use linkme::distributed_slice;
use log::debug;
use ron::Value;
use serde::{Deserialize, Serialize};
use tokio::{
    runtime::Handle,
    sync::{mpsc::UnboundedSender, Mutex},
};

use dynisland_core::{
    base_module::{
        ActivityMap, DynamicActivity, Module, ModuleConfig, Producer, PropertyUpdate,
        UIServerCommand, MODULES,
    },
    cast_dyn_any,
    graphics::{
        activity_widget::widget::{ActivityMode, ActivityWidget},
        widgets::{rolling_number::RollingNumber, scrolling_label::ScrollingLabel, Orientation},
    },
};

//add to modules to be loaded
#[distributed_slice(MODULES)]
static EXAMPLE_MODULE: fn(UnboundedSender<UIServerCommand>, Option<Value>) -> Box<dyn Module> =
    ExampleModule::new;

/// for now this is just used to test new code
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExampleConfig {
    #[serde(default)]
    pub int: i32,
    #[serde(default)]
    pub string: String,
    #[serde(default)]
    pub vec: Vec<String>,

    pub duration: u64,

    #[serde(
        deserialize_with = "Bezier::from_string_or_struct",
        default = "default_bezier"
    )]
    pub translate_prev: Bezier,
    #[serde(
        deserialize_with = "Bezier::from_string_or_struct",
        default = "default_bezier"
    )]
    pub scale_prev: Bezier,
    #[serde(
        deserialize_with = "Bezier::from_string_or_struct",
        default = "default_bezier"
    )]
    pub opacity_prev: Bezier,
    #[serde(
        deserialize_with = "Bezier::from_string_or_struct",
        default = "default_bezier"
    )]
    pub blur_prev: Bezier,

    #[serde(
        deserialize_with = "Bezier::from_string_or_struct",
        default = "default_bezier"
    )]
    pub translate_next: Bezier,
    #[serde(
        deserialize_with = "Bezier::from_string_or_struct",
        default = "default_bezier"
    )]
    pub scale_next: Bezier,
    #[serde(
        deserialize_with = "Bezier::from_string_or_struct",
        default = "default_bezier"
    )]
    pub opacity_next: Bezier,
    #[serde(
        deserialize_with = "Bezier::from_string_or_struct",
        default = "default_bezier"
    )]
    pub blur_next: Bezier,
}

fn default_bezier() -> Bezier {
    Bezier::from_str("linear").unwrap()
}

impl ModuleConfig for ExampleConfig {}
impl Default for ExampleConfig {
    fn default() -> Self {
        Self {
            int: 0,
            string: String::from("Example1"),
            vec: vec![String::from("Example2"), String::from("Example3")],
            duration: 400,
            translate_prev: default_bezier(),
            scale_prev: default_bezier(),
            opacity_prev: default_bezier(),
            blur_prev: default_bezier(),
            translate_next: default_bezier(),
            scale_next: default_bezier(),
            opacity_next: default_bezier(),
            blur_next: default_bezier(),
        }
    }
}
pub struct ExampleModule {
    name: String,
    app_send: UnboundedSender<UIServerCommand>,
    prop_send: UnboundedSender<PropertyUpdate>,
    registered_activities: ActivityMap,
    registered_producers: Arc<Mutex<HashSet<Producer>>>,
    config: ExampleConfig,
}

#[async_trait(?Send)]
impl Module for ExampleModule {
    fn new(app_send: UnboundedSender<UIServerCommand>, config: Option<Value>) -> Box<dyn Module> {
        let conf = match config {
            Some(value) => value.into_rust().expect("failed to parse config"),
            None => ExampleConfig::default(),
        };
        let registered_activities = Rc::new(Mutex::new(HashMap::<
            String,
            Rc<Mutex<DynamicActivity>>,
        >::new()));

        let prop_send = ExampleModule::spawn_property_update_loop(&registered_activities);

        Box::new(Self {
            name: "ExampleModule".to_string(),
            app_send,
            prop_send,
            registered_activities,
            registered_producers: Arc::new(Mutex::new(HashSet::new())),
            config: conf,
        })
    }

    // fn spawn_property_update_loop(registered_activities:&ActivityMap) -> UnboundedSender<PropertyUpdate> {
    //     //create ui property update channel
    //     let (prop_send, mut prop_recv) = tokio::sync::mpsc::unbounded_channel::<PropertyUpdate>();
    //     let activities=registered_activities.clone();
    //     glib::MainContext::default().spawn_local(async move {
    //         //start data consumer
    //         while let Some(res) = prop_recv.recv().await {
    //             if res.activity_id == "*" {
    //                 for activity in activities.lock().await.values() {
    //                     match activity.lock().await.get_subscribers(&res.property_name) {
    //                         core::result::Result::Ok(subs) => {
    //                             for sub in subs {
    //                                 sub(&*res.value);
    //                             }
    //                         }
    //                         Err(_err) => {
    //                             // error!("{}", err)
    //                         }
    //                     }
    //                 }
    //             } else {
    //                 match activities.lock().await.get(&res.activity_id) {
    //                     Some(activity) => {
    //                         match activity.lock().await.get_subscribers(&res.property_name) {
    //                             core::result::Result::Ok(subs) => {
    //                                 for sub in subs {
    //                                     sub(&*res.value);
    //                                 }
    //                             }
    //                             Err(_err) => {
    //                                 // error!("{}", err)
    //                             }
    //                         }
    //                     }
    //                     None => {
    //                         error!("activity {} not found on ExampleModule", res.activity_id);
    //                     }
    //                 }
    //             }
    //         }
    //     });
    //     prop_send
    // }

    fn get_name(&self) -> &str {
        &self.name
    }
    fn get_config(&self) -> &dyn ModuleConfig {
        &self.config
    }

    fn get_registered_activities(&self) -> ActivityMap {
        self.registered_activities.clone()
    }

    async fn register_activity(&self, activity: Rc<Mutex<DynamicActivity>>) {
        let mut reg = self.registered_activities.lock().await;
        let activity_id = activity.lock().await.get_identifier();
        if reg.contains_key(&activity_id) {
            panic!("activity {} was already registered", activity_id);
        }
        reg.insert(activity_id, activity.clone());
    }
    async fn unregister_activity(&self, activity: &str) {
        self.registered_activities
            .lock()
            .await
            .remove(activity)
            .expect("activity isn't registered");
    }

    fn get_registered_producers(&self) -> Arc<Mutex<HashSet<Producer>>> {
        self.registered_producers.clone()
    }

    async fn register_producer(&self, producer: Producer) {
        self.registered_producers.lock().await.insert(producer);
    }

    fn get_prop_send(&self) -> UnboundedSender<PropertyUpdate> {
        self.prop_send.clone()
    }

    fn init(&self) {
        let app_send = self.app_send.clone();
        let name = self.name.clone();
        let prop_send = self.prop_send.clone();
        glib::MainContext::default().spawn_local(async move {
            //create activity
            let activity = Rc::new(Mutex::new(Self::get_activity(
                prop_send,
                "exampleActivity1",
            )));

            //register activity and data producer
            app_send
                .send(UIServerCommand::AddActivity(name.clone(), activity.clone()))
                .unwrap();
            app_send
                .send(UIServerCommand::AddProducer(
                    name,
                    Self::producer as Producer,
                ))
                .unwrap();
        });
    }

    fn parse_config(&mut self, config: Value) -> Result<()> {
        self.config = config
            .into_rust()
            .with_context(|| "failed to parse config")
            .unwrap();
        Ok(())
    }
}

fn update_config(config: &ExampleConfig, activities: ActivityMap) {
    let compact_mode = activities
        .blocking_lock()
        .get("exampleActivity1")
        .unwrap()
        .blocking_lock()
        .get_activity_widget()
        .compact_mode()
        .unwrap();
    let rn1 = compact_mode
        .clone()
        .downcast::<gtk::EventBox>()
        .unwrap()
        .children()
        .first()
        .unwrap()
        .clone()
        .downcast::<gtk::Box>()
        .unwrap()
        .children()
        .first()
        .unwrap()
        .clone()
        .downcast::<RollingNumber>()
        .unwrap();
    let rn2 = compact_mode
        .clone()
        .downcast::<gtk::EventBox>()
        .unwrap()
        .children()
        .first()
        .unwrap()
        .clone()
        .downcast::<gtk::Box>()
        .unwrap()
        .children()
        .get(1)
        .unwrap()
        .clone()
        .downcast::<RollingNumber>()
        .unwrap();
    let rn_list = vec![rn1, rn2]; //
    for rn in rn_list {
        rn.set_translate_prev_transition(Box::new(config.translate_prev), true)
            .unwrap();
        rn.set_scale_prev_transition(Box::new(config.scale_prev), true)
            .unwrap();
        rn.set_opacity_prev_transition(Box::new(config.opacity_prev), true)
            .unwrap();
        rn.set_blur_prev_transition(Box::new(config.blur_prev), true)
            .unwrap();
        rn.set_translate_next_transition(Box::new(config.translate_next), true)
            .unwrap();
        rn.set_scale_next_transition(Box::new(config.scale_next), true)
            .unwrap();
        rn.set_opacity_next_transition(Box::new(config.opacity_next), true)
            .unwrap();
        rn.set_blur_next_transition(Box::new(config.blur_next), true)
            .unwrap();
        rn.set_transition_duration(config.duration, true).unwrap();
    }
}

impl ExampleModule {
    //TODO add reference to module and recieve messages from main
    #[allow(unused_variables)]
    fn producer(
        activities: ActivityMap,
        rt: &Handle,
        _app_send: UnboundedSender<UIServerCommand>,
        _prop_send: UnboundedSender<PropertyUpdate>,
        config: &dyn ModuleConfig,
    ) {
        //data producer
        let config: &ExampleConfig = cast_dyn_any!(config, ExampleConfig).unwrap();
        update_config(config, activities.clone());
        //TODO shouldn't be blocking locks, maybe execute async with glib::MainContext
        let act = activities.blocking_lock();
        let mode = act
            .get("exampleActivity1")
            .unwrap()
            .blocking_lock()
            .get_property("mode")
            .unwrap();
        // let label = act
        //     .get("exampleActivity1")
        //     .unwrap()
        //     .blocking_lock()
        //     .get_property("comp-label")
        //     .unwrap();
        let scrolling_enabled = act
            .get("exampleActivity1")
            .unwrap()
            .blocking_lock()
            .get_property("scrolling-transition-enabled")
            .unwrap();
        let scrolling_text = act
            .get("exampleActivity1")
            .unwrap()
            .blocking_lock()
            .get_property("scrolling-label-text")
            .unwrap();
        let rolling_number = act
            .get("exampleActivity1")
            .unwrap()
            .blocking_lock()
            .get_property("rolling-number")
            .unwrap();
        // label.blocking_lock().set(config.string.clone()).unwrap();

        if let Some(widget) = act
            .get("exampleActivity1")
            .unwrap()
            .blocking_lock()
            .get_activity_widget()
            .current_widget()
        {
            //raise window associated to widget if it has one, this enables events on the active mode widget
            if let Some(window) = widget.window() {
                window.raise();
            }
        }

        // let activity = Arc::new(Mutex::new(Self::get_activity(
        //     prop_send.clone(),
        //     "exampleActivity2",
        // )));
        // app_send
        //     .send(UIServerCommand::AddActivity(
        //         "ExampleModule".to_string(),
        //         activity,
        //     ))
        //     .unwrap();

        let config = config.clone();
        // debug!("starting task");
        rt.spawn(async move {
            // debug!("task started");
            // mode.lock().await.set(ActivityMode::Minimal).unwrap();
            loop {
                rolling_number.lock().await.set('0').unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(config.duration + 500)).await;
                rolling_number.lock().await.set('1').unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(config.duration + 500)).await;
                rolling_number.lock().await.set('2').unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(config.duration + 500)).await;
                rolling_number.lock().await.set('3').unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(config.duration + 500)).await;
                rolling_number.lock().await.set('4').unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(config.duration + 500)).await;
                // scrolling_enabled.lock().await.set(false).unwrap();
                // scrolling_text
                //     .lock()
                //     .await
                //     .set("Hello long text, very long text. Hello long text, very long text.    end".to_string())
                //     .unwrap();
                // tokio::time::sleep(tokio::time::Duration::from_millis(6000)).await;
                // scrolling_enabled.lock().await.set(true).unwrap();
                // scrolling_text
                //     .lock()
                //     .await
                //     .set("Hello shorterer e e e e text e.    end".to_string())
                //     .unwrap();
                // tokio::time::sleep(tokio::time::Duration::from_millis(17000)).await;
                // mode.lock().await.set(ActivityMode::Minimal).unwrap();
                // tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;

                // mode.lock().await.set(ActivityMode::Compact).unwrap();
                // tokio::time::sleep(tokio::time::Duration::from_millis(2500)).await;
                // let old_label_val;
                // {
                //     let label_val = label.lock().await;
                //     let str_val: &String = cast_dyn_any!(label_val.get(), String).unwrap();
                //     old_label_val = str_val.clone();
                // }

                // tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                // label.lock().await.set("sdkjvksdv1 tryt etvcbssrfh".to_string()).unwrap();
                // tokio::time::sleep(tokio::time::Duration::from_millis(1200)).await;
                // label.lock().await.set("fghn".to_string()).unwrap();
                // tokio::time::sleep(tokio::time::Duration::from_millis(1200)).await;

                // label.lock().await.set(old_label_val).unwrap();
                // tokio::time::sleep(tokio::time::Duration::from_millis(1200)).await;

                // prop_send
                //     .send(PropertyUpdate {
                //         activity_id: "*".to_string(),
                //         property_name: "mode".to_string(),
                //         value: Box::new(ActivityMode::Compact),
                //     })
                //     .unwrap();
                // mode.lock().await.set(ActivityMode::Expanded).unwrap();

                // tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
                // prop_send
                //     .send(PropertyUpdate {
                //         activity_id: "*".to_string(),
                //         property_name: "mode".to_string(),
                //         value: Box::new(ActivityMode::Expanded),
                //     })
                //     .unwrap();
                // mode.lock().await.set(ActivityMode::Compact).unwrap();
                // tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
                // mode.lock().await.set(ActivityMode::Expanded).unwrap();
                // tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
                // mode.lock().await.set(ActivityMode::Overlay).unwrap();
                // tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
                // mode.lock().await.set(ActivityMode::Expanded).unwrap();
                // tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
            }
        });
    }

    fn get_activity(
        prop_send: tokio::sync::mpsc::UnboundedSender<PropertyUpdate>,
        name: &str,
    ) -> DynamicActivity {
        let mut activity = DynamicActivity::new(prop_send, name);

        //create activity widget
        let mut activity_widget = activity.get_activity_widget();
        Self::set_act_widget(&mut activity_widget);
        //get widgets
        let background = Self::get_bg();
        let minimal = Self::get_minimal();
        let compact = Self::get_compact();
        let expanded = Self::get_expanded();
        let overlay = Self::get_overlay();

        //load widgets in the activity widget
        activity_widget.add(&background);
        activity_widget.set_minimal_mode(&minimal);
        activity_widget.set_compact_mode(&compact);
        activity_widget.set_expanded_mode(&expanded);
        activity_widget.set_overlay_mode(&overlay);

        // activity_widget.connect_mode_notify(|f| {
        // let l = f.mode();
        // debug!("Changed mode: {:?}", l);
        // });

        activity.set_activity_widget(activity_widget.clone());

        activity
            .add_dynamic_property("mode", ActivityMode::Minimal)
            .unwrap();
        // activity
        //     .add_dynamic_property("comp-label", "compact".to_string())
        //     .unwrap();
        activity
            .add_dynamic_property("scrolling-transition-enabled", true)
            .unwrap();
        activity
            .add_dynamic_property("scrolling-label-text", "Hello, World".to_string())
            .unwrap();
        activity
            .add_dynamic_property("rolling-number", '0')
            .unwrap();

        let mode = activity.get_property("mode").unwrap();

        let minimal_cl = minimal.clone();
        activity
            .subscribe_to_property("scrolling-transition-enabled", move |new_value| {
                let real_value = cast_dyn_any!(new_value, bool).unwrap();
                debug!("enabled changed:{real_value}");
                minimal_cl
                    .clone()
                    .downcast::<gtk::EventBox>()
                    .unwrap()
                    .children()
                    .first()
                    .unwrap()
                    .clone()
                    .downcast::<gtk::Box>()
                    .unwrap()
                    .children()
                    .first()
                    .unwrap()
                    .clone()
                    .downcast::<ScrollingLabel>()
                    .unwrap()
                    .set_transition_enabled(real_value);
            })
            .unwrap();

        let minimal_cl = minimal.clone();
        activity
            .subscribe_to_property("scrolling-label-text", move |new_value| {
                let real_value = cast_dyn_any!(new_value, String).unwrap();
                debug!("text changed:{real_value}");
                minimal_cl
                    .clone()
                    .downcast::<gtk::EventBox>()
                    .unwrap()
                    .children()
                    .first()
                    .unwrap()
                    .clone()
                    .downcast::<gtk::Box>()
                    .unwrap()
                    .children()
                    .first()
                    .unwrap()
                    .clone()
                    .downcast::<ScrollingLabel>()
                    .unwrap()
                    .set_text(real_value.as_str());
            })
            .unwrap();

        minimal.add_events(gdk::EventMask::BUTTON_RELEASE_MASK);
        let m1 = mode.clone();
        minimal.connect_button_release_event(move |_wid, ev| {
            if let gdk::EventType::ButtonRelease = ev.event_type() {
                debug!("min");
                let m1 = m1.clone();
                match ev.button() {
                    gdk::BUTTON_PRIMARY => {
                        glib::MainContext::default().spawn_local(async move {
                            m1.lock().await.set(ActivityMode::Compact).unwrap();
                        });
                    }
                    gdk::BUTTON_SECONDARY => {
                        glib::MainContext::default().spawn_local(async move {
                            m1.lock().await.set(ActivityMode::Overlay).unwrap();
                        });
                    }
                    _ => {}
                }
            }
            glib::Propagation::Proceed
        });

        compact.add_events(gdk::EventMask::BUTTON_RELEASE_MASK);
        let m1 = mode.clone();
        compact.connect_button_release_event(move |_wid, ev| {
            if let gdk::EventType::ButtonRelease = ev.event_type() {
                debug!("comp");
                let m1 = m1.clone();
                match ev.button() {
                    gdk::BUTTON_PRIMARY => {
                        glib::MainContext::default().spawn_local(async move {
                            m1.lock().await.set(ActivityMode::Expanded).unwrap();
                        });
                    }
                    gdk::BUTTON_SECONDARY => {
                        glib::MainContext::default().spawn_local(async move {
                            m1.lock().await.set(ActivityMode::Minimal).unwrap();
                        });
                    }
                    _ => {}
                }
            }
            glib::Propagation::Proceed
        });

        expanded.add_events(gdk::EventMask::BUTTON_RELEASE_MASK);
        let m1 = mode.clone();
        expanded.connect_button_release_event(move |_wid, ev| {
            if let gdk::EventType::ButtonRelease = ev.event_type() {
                debug!("exp");
                let m1 = m1.clone();
                match ev.button() {
                    gdk::BUTTON_PRIMARY => {
                        glib::MainContext::default().spawn_local(async move {
                            m1.lock().await.set(ActivityMode::Overlay).unwrap();
                        });
                    }
                    gdk::BUTTON_SECONDARY => {
                        glib::MainContext::default().spawn_local(async move {
                            m1.lock().await.set(ActivityMode::Compact).unwrap();
                        });
                    }
                    _ => {}
                }
            }
            glib::Propagation::Proceed
        });

        overlay.add_events(gdk::EventMask::BUTTON_RELEASE_MASK);
        let m1 = mode.clone();
        overlay.connect_button_release_event(move |_wid, ev| {
            if let gdk::EventType::ButtonRelease = ev.event_type() {
                debug!("exp");
                let m1 = m1.clone();
                match ev.button() {
                    gdk::BUTTON_PRIMARY => {
                        glib::MainContext::default().spawn_local(async move {
                            m1.lock().await.set(ActivityMode::Minimal).unwrap();
                        });
                    }
                    gdk::BUTTON_SECONDARY => {
                        glib::MainContext::default().spawn_local(async move {
                            m1.lock().await.set(ActivityMode::Expanded).unwrap();
                        });
                    }
                    _ => {}
                }
            }
            glib::Propagation::Proceed
        });

        background.add_events(gdk::EventMask::BUTTON_RELEASE_MASK);
        let m1 = mode.clone();
        background.connect_button_release_event(move |_wid, ev| {
            if let gdk::EventType::ButtonRelease = ev.event_type() {
                // debug!("bg");
                let m1 = m1.clone();
                match ev.button() {
                    gdk::BUTTON_PRIMARY => {
                        glib::MainContext::default().spawn_local(async move {
                            let mode_g = m1.lock().await;
                            let mode = *cast_dyn_any!(mode_g.get(), ActivityMode).unwrap();
                            drop(mode_g);

                            match mode {
                                ActivityMode::Minimal => {
                                    m1.lock().await.set(ActivityMode::Compact).unwrap();
                                }
                                ActivityMode::Compact => {
                                    m1.lock().await.set(ActivityMode::Expanded).unwrap();
                                }
                                ActivityMode::Expanded => {
                                    m1.lock().await.set(ActivityMode::Overlay).unwrap();
                                }
                                ActivityMode::Overlay => {
                                    m1.lock().await.set(ActivityMode::Minimal).unwrap();
                                }
                            }
                        });
                    }
                    gdk::BUTTON_SECONDARY => {
                        glib::MainContext::default().spawn_local(async move {
                            let mode_g = m1.lock().await;
                            let mode = *cast_dyn_any!(mode_g.get(), ActivityMode).unwrap();
                            drop(mode_g);

                            match mode {
                                ActivityMode::Minimal => {
                                    m1.lock().await.set(ActivityMode::Overlay).unwrap();
                                }
                                ActivityMode::Compact => {
                                    m1.lock().await.set(ActivityMode::Minimal).unwrap();
                                }
                                ActivityMode::Expanded => {
                                    m1.lock().await.set(ActivityMode::Compact).unwrap();
                                }
                                ActivityMode::Overlay => {
                                    m1.lock().await.set(ActivityMode::Expanded).unwrap();
                                }
                            }
                        });
                    }
                    _ => {}
                }
            }
            glib::Propagation::Proceed
        });

        //set mode when updated
        activity
            .subscribe_to_property("mode", move |new_value| {
                let real_value = cast_dyn_any!(new_value, ActivityMode).unwrap();
                activity_widget.set_mode(real_value);
            })
            .unwrap();

        activity
            .subscribe_to_property("rolling-number", move |new_value| {
                let real_value = cast_dyn_any!(new_value, char).unwrap();
                compact //FIXME WTF is this, i need to change it, maybe with a macro
                    .clone()
                    .downcast::<gtk::EventBox>()
                    .unwrap()
                    .children()
                    .first()
                    .unwrap()
                    .clone()
                    .downcast::<gtk::Box>()
                    .unwrap()
                    .children()
                    .first()
                    .unwrap()
                    .clone()
                    .downcast::<RollingNumber>()
                    .unwrap()
                    .set_number(real_value);
                compact //FIXME WTF is this, i need to change it, maybe with a macro
                    .clone()
                    .downcast::<gtk::EventBox>()
                    .unwrap()
                    .children()
                    .first()
                    .unwrap()
                    .clone()
                    .downcast::<gtk::Box>()
                    .unwrap()
                    .children()
                    .get(1)
                    .unwrap()
                    .clone()
                    .downcast::<RollingNumber>()
                    .unwrap()
                    .set_number(real_value);
            })
            .unwrap();

        // //set label when updated
        // activity
        //     .subscribe_to_property("comp-label", move |new_value| {
        //         let real_value = cast_dyn_any!(new_value, String).unwrap();
        //         compact //FIXME WTF is this, i need to change it, maybe with a macro
        //             .clone()
        //             .downcast::<gtk::EventBox>()
        //             .unwrap()
        //             .children()
        //             .first()
        //             .unwrap()
        //             .clone()
        //             .downcast::<gtk::Box>()
        //             .unwrap()
        //             .children()
        //             .first()
        //             .unwrap()
        //             .clone()
        //             .downcast::<gtk::Label>()
        //             .unwrap()
        //             .set_label(real_value);
        //     })
        //     .unwrap();

        activity
    }

    fn set_act_widget(activity_widget: &mut ActivityWidget) {
        activity_widget.set_vexpand(false);
        activity_widget.set_hexpand(false);
        activity_widget.set_valign(gtk::Align::Start);
        activity_widget.set_halign(gtk::Align::Center);
        // activity_widget.set_transition_duration(2000, true).unwrap();
        // activity_widget.style_context().add_class("overlay");
    }

    fn get_bg() -> gtk::Widget {
        let background = gtk::Label::builder()
            .label("")
            .halign(gtk::Align::Start)
            .valign(gtk::Align::Start)
            .build();
        let background = gtk::Box::builder()
            // .height_request(40)
            // .width_request(100)
            .valign(gtk::Align::Start)
            .halign(gtk::Align::Center)
            .vexpand(true)
            .hexpand(true)
            // .above_child(false) //Allows events on children (like buttons)
            .child(&background)
            .build();

        let background = gtk::EventBox::builder()
            // .height_request(40)
            // .width_request(100)
            .valign(gtk::Align::Start)
            .halign(gtk::Align::Center)
            .vexpand(false)
            .hexpand(false)
            .above_child(false) //Allows events on children (like buttons)
            .child(&background)
            .build();

        background.upcast()
    }

    fn get_minimal() -> gtk::Widget {
        let minimal = gtk::Box::builder()
            // .height_request(40)
            .width_request(140)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .vexpand(false)
            .hexpand(false)
            .homogeneous(true)
            .build();
        minimal.set_margin_start(20);
        minimal.set_margin_end(20);

        // let btn = gtk::Label::builder()
        //     .label("m")
        //     .halign(gtk::Align::Center)
        //     .valign(gtk::Align::Center)
        //     .hexpand(true)
        //     .build();
        // minimal.add(&btn);

        let scroll_label = ScrollingLabel::new();
        scroll_label.set_max_height(40);
        scroll_label.set_max_width(140); // ?? should be width+internal margins for vertical
        scroll_label.set_orientation(Orientation::Horizontal);
        scroll_label.set_transition_roll(true);
        scroll_label.set_text("valueasdfvasdfasdfasdfasfd");
        scroll_label.set_transition_speed(30, true).unwrap();
        scroll_label.set_timeout_duration(2000, true).unwrap();
        // scroll_label.set_transition(Box::new(Bezier::from_str("ease-in-out").unwrap()), true).unwrap();

        scroll_label.inner_label().set_margin_start(10);
        scroll_label.inner_label().set_margin_end(30);

        minimal.add(&scroll_label);

        let minimal = gtk::EventBox::builder()
            .height_request(40)
            // .width_request(100)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .vexpand(false)
            .hexpand(false)
            .above_child(false) //Allows events on children (like buttons)
            .child(&minimal)
            .build();
        // minimal.parent_window().unwrap().set_keep_above(true);
        minimal.upcast()
    }

    fn get_compact() -> gtk::Widget {
        let compact = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .height_request(40)
            .width_request(280)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .vexpand(false)
            .hexpand(false)
            .build();
        // compact.add(
        //     &gtk::Label::builder()
        //         .label("compact")
        //         .halign(gtk::Align::Center)
        //         .valign(gtk::Align::Center)
        //         .hexpand(true)
        //         .build(),
        // );
        let rn1 = RollingNumber::new();
        rn1.set_transition_duration(400, true).unwrap();
        rn1.set_number('0');
        rn1.set_valign(gtk::Align::Center);
        rn1.set_halign(gtk::Align::Center);
        compact.add(&rn1);

        let rn2 = RollingNumber::new();
        rn2.set_transition_duration(400, true).unwrap();
        rn2.set_transition_delay(150, true).unwrap();
        rn2.set_number('0');
        rn2.set_valign(gtk::Align::Center);
        rn2.set_halign(gtk::Align::Center);
        compact.add(&rn2);

        let compact = gtk::EventBox::builder()
            .height_request(40)
            .width_request(280)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .vexpand(true)
            .hexpand(false)
            .child(&compact)
            .build();
        compact.upcast()
    }

    fn get_expanded() -> gtk::Widget {
        let expanded = gtk::Box::builder()
            .height_request(400)
            .width_request(500)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .vexpand(false)
            .hexpand(false)
            .build();

        expanded.add(
            &gtk::Label::builder()
                .label("Expanded label,\n Hello Hello")
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .hexpand(true)
                .build(),
        );
        let expanded = gtk::EventBox::builder()
            .height_request(400)
            .width_request(500)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .vexpand(false)
            .hexpand(false)
            .child(&expanded)
            .build();
        expanded.upcast()
    }

    fn get_overlay() -> gtk::Widget {
        let expanded = gtk::Box::builder()
            .height_request(1080)
            .width_request(1920)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .vexpand(false)
            .hexpand(false)
            .build();

        expanded.add(
            &gtk::Label::builder()
                .label("Overlay label,\n Hello Hello \n Hello Hello")
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .hexpand(true)
                .build(),
        );
        expanded.upcast()
    }
}

// Example2 ------------------------------------------------------------------------------------------------------
// ---------------------------------------------------------------------------------------------------------------
// ---------------------------------------------------------------------------------------------------------------

// pub struct ExampleModule2 {
//     name: String,
//     app_send: UnboundedSender<UIServerCommand>,
//     registered_activities: ActivityMap,
//     registered_producers: Arc<Mutex<HashSet<Producer>>>,
//     config: ExampleConfig,
// }

// #[async_trait(?Send)]
// impl Module for ExampleModule2 {
//     fn new(app_send: UnboundedSender<UIServerCommand>, config: Option<Value>) -> Box<Self> {
//         let conf = match config {
//             Some(value) => value.into_rust().expect("failed to parse config"),
//             None => ExampleConfig::default(),
//         };
//         Box::new(Self {
//             name: "ExampleModule2".to_string(),
//             app_send,
//             registered_activities: Arc::new(Mutex::new(HashMap::new())),
//             registered_producers: Arc::new(Mutex::new(HashSet::new())),
//             config: conf,
//         })
//     }

//     fn get_name(&self) -> &str {
//         &self.name
//     }
//     fn get_config(&self) -> &dyn ModuleConfig {
//         &self.config
//     }

//     fn get_registered_activities(
//         &self,
//     ) -> Arc<Mutex<HashMap<String, Arc<Mutex<DynamicActivity>>>>> {
//         self.registered_activities.clone()
//     }
//     async fn register_activity(&self, activity: Arc<Mutex<DynamicActivity>>) {
//         self.registered_activities
//             .lock()
//             .await
//             .insert(activity.lock().await.get_identifier(), activity.clone());
//     }

//     fn get_registered_producers(&self) -> Arc<Mutex<HashSet<Producer>>> {
//         self.registered_producers.clone()
//     }

//     async fn register_producer(&self, producer: Producer) {
//         self.registered_producers.lock().await.insert(producer);
//     }

//     fn init(&self) {

//         //create ui channel
//         let (prop_send, mut prop_recv) = tokio::sync::mpsc::unbounded_channel::<PropertyUpdate>();

//         let app_send = self.app_send.clone();
//         let name = self.name.clone();
//         glib::MainContext::default().spawn_local(async move {
//             //create activity
//             let activity = Arc::new(Mutex::new(Self::get_activity(prop_send)));

//             //register activity and data producer
//             app_send
//                 .send(UIServerCommand::AddActivity(name.clone(), activity.clone()))
//                 .unwrap();

//             app_send
//                 .send(UIServerCommand::AddProducer(name, Self::producer))
//                 .unwrap();

//             //start data consumer
//             while let Some(res) = prop_recv.recv().await {
//                 match activity.lock().await.get_subscribers(&res.0) {
//                     core::result::Result::Ok(subs) => {
//                         for sub in subs {
//                             sub(&*res.1);
//                         }
//                     }
//                     Err(err) => error!("{}", err),
//                 }
//             }
//         });
//     }
//     fn parse_config(&mut self, config: Value) -> Result<()> {
//         self.config = config
//             .into_rust()
//             .with_context(|| "failed to parse config")
//             .unwrap();
//         Ok(())
//     }
// }

// impl ExampleModule2 {
//     fn producer(
//         activities: ActivityMap,
//         rt: &Handle,
//         _app_send: UnboundedSender<UIServerCommand>,
//         config: &dyn ModuleConfig,
//     ) {
//         //data producer
//         let _config: &ExampleConfig = cast_dyn_any!(config, ExampleConfig).unwrap();
//         let act = activities.blocking_lock();
//         let mode = act
//             .get("exampleActivity2")
//             .unwrap()
//             .blocking_lock()
//             .get_property("mode")
//             .unwrap();
//         let label = act
//             .get("exampleActivity2")
//             .unwrap()
//             .blocking_lock()
//             .get_property("comp-label")
//             .unwrap();

//         rt.spawn(async move {
//             loop {
//                 tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
//                 mode.lock().await.set(ActivityMode::Minimal).unwrap();

//                 tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
//                 mode.lock().await.set(ActivityMode::Compact).unwrap();
//                 let old_label_val;
//                 {
//                     let label_val = label.lock().await;
//                     let str_val: &String = (label_val.get() as &dyn std::any::Any)
//                         .downcast_ref()
//                         .unwrap();
//                     old_label_val = str_val.clone();
//                 }

//                 tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
//                 label.lock().await.set("sdkjvksdv2".to_string()).unwrap();
//                 tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
//                 label.lock().await.set("fghn2".to_string()).unwrap();
//                 tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

//                 label.lock().await.set(old_label_val).unwrap();

//                 tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
//                 mode.lock().await.set(ActivityMode::Expanded).unwrap();

//                 tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
//                 mode.lock().await.set(ActivityMode::Compact).unwrap();
//             }
//         });
//     }

//     fn get_activity(
//         prop_send: tokio::sync::mpsc::UnboundedSender<PropertyUpdate>,
//     ) -> DynamicActivity {
//         let mut activity = DynamicActivity::new(prop_send, "exampleActivity2");

//         //create activity widget
//         let mut activity_widget=activity.get_activity_widget();
//         Self::set_act_widget(&mut activity_widget);
//         //get widgets
//         let background = Self::get_bg();
//         let minimal = Self::get_minimal();
//         let compact = Self::get_compact();
//         let expanded = Self::get_expanded();

//         //load widgets in the activity widget
//         activity_widget.add(&background);
//         activity_widget.set_minimal_mode(&minimal);
//         activity_widget.set_compact_mode(&compact);
//         activity_widget.set_expanded_mode(&expanded);

//         // activity_widget.connect_mode_notify(|f| {
//         //     let l = f.mode();
//         //     debug!("Changed mode: {:?}", l);
//         // });
//         activity.set_activity_widget(activity_widget.clone());

//         activity
//             .add_dynamic_property("mode", ActivityMode::Minimal)
//             .unwrap();
//         //set mode when updated
//         activity
//             .subscribe_to_property("mode", move |new_value| {
//                 let real_value = cast_dyn_any!(new_value, ActivityMode).unwrap();
//                 activity_widget.set_mode(real_value);
//             })
//             .unwrap();

//         activity
//             .add_dynamic_property("comp-label", "compact".to_string())
//             .unwrap();
//         //set label when updated
//         activity
//             .subscribe_to_property("comp-label", move |new_value| {
//                 let real_value = cast_dyn_any!(new_value, String).unwrap();
//                 compact
//                     .clone()
//                     .downcast::<gtk::Box>()
//                     .unwrap()
//                     .children()
//                     .get(0)
//                     .unwrap()
//                     .clone()
//                     .downcast::<gtk::Label>()
//                     .unwrap()
//                     .set_label(real_value);
//             })
//             .unwrap();

//         activity
//     }

//     fn set_act_widget(activity_widget: &mut ActivityWidget){
//         activity_widget.set_vexpand(false);
//         activity_widget.set_hexpand(false);
//         activity_widget.set_valign(gtk::Align::Start);
//         activity_widget.set_halign(gtk::Align::Center);
//         activity_widget.local_css_context().set_transition_duration(1000).unwrap();
//         activity_widget.style_context().add_class("overlay");
//     }

//     fn get_bg() -> gtk::Widget {
//         let background = gtk::Label::builder()
//             .label("")
//             .halign(gtk::Align::Start)
//             .valign(gtk::Align::Start)
//             .build();
//         background.upcast()
//     }

//     fn get_minimal() -> gtk::Widget {;
//                 .halign(gtk::Align::Center)
//                 .valign(gtk::Align::Center)
//                 .hexpand(true)
//                 .build(),
//         );
//         minimal.upcast()
//     }

//     fn get_compact() -> gtk::Widget {
//         let compact = gtk::Box::builder()
//             .height_request(40)
//             .width_request(170)
//             .valign(gtk::Align::Center)
//             .halign(gtk::Align::Center)
//             .vexpand(false)
//             .hexpand(false)
//             .build();

//         compact.add(
//             &gtk::Label::builder()
//                 .label("compact2")
//                 .halign(gtk::Align::Center)
//                 .valign(gtk::Align::Center)
//                 .hexpand(true)
//                 .build(),
//         );
//         compact.upcast()
//     }

//     fn get_expanded() -> gtk::Widget {
//         let expanded = gtk::Box::builder()
//             .height_request(100)
//             .width_request(350)
//             .valign(gtk::Align::Center)
//             .halign(gtk::Align::Center)
//             .vexpand(false)
//             .hexpand(false)
//             .build();

//         expanded.add(
//             &gtk::Label::builder()
//                 .label("Expanded label2,\n Hello Hello Hello")
//                 .halign(gtk::Align::Center)
//                 .valign(gtk::Align::Center)
//                 .hexpand(true)
//                 .build(),
//         );
//         expanded.upcast()
//     }
// }
