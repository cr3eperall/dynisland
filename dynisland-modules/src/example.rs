use std::{any::Any, collections::HashSet, rc::Rc, sync::Arc, vec};

use anyhow::{Context, Ok, Result};
use async_trait::async_trait;
use gtk::{prelude::*, GestureClick, Label};
use linkme::distributed_slice;
use ron::Value;
use serde::{Deserialize, Serialize};
use tokio::{
    runtime::Handle,
    sync::{mpsc::UnboundedSender, Mutex},
};

use dynisland_core::{
    base_module::{
        ActivityMap, DynamicActivity, Module, ModuleDefinition, Producer, PropertyUpdate,
        UIServerCommand, MODULES,
    },
    cast_dyn_any,
    graphics::{
        activity_widget::{imp::ActivityMode, ActivityWidget},
        widgets::{rolling_char::RollingChar, scrolling_label::ScrollingLabel},
    },
};

pub const NAME: &str = "ExampleModule";

//add to modules to be loaded
#[distributed_slice(MODULES)]
static EXAMPLE_MODULE: ModuleDefinition = (NAME, ExampleModule::new);

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
}

// impl ModuleConfig for ExampleConfig {}
impl Default for ExampleConfig {
    fn default() -> Self {
        Self {
            int: 0,
            string: String::from("Example1"),
            vec: vec![String::from("Example2"), String::from("Example3")],
            duration: 400,
        }
    }
}
pub struct ExampleModule {
    // name: String,
    app_send: UnboundedSender<UIServerCommand>,
    prop_send: UnboundedSender<PropertyUpdate>,
    registered_activities: Rc<Mutex<ActivityMap>>,
    registered_producers: Arc<Mutex<HashSet<Producer>>>,
    config: ExampleConfig,
}

#[async_trait(?Send)]
impl Module for ExampleModule {
    fn new(app_send: UnboundedSender<UIServerCommand>) -> Box<dyn Module> {
        let registered_activities = Rc::new(Mutex::new(ActivityMap::new()));

        let prop_send = ExampleModule::spawn_property_update_loop(&registered_activities);

        Box::new(Self {
            // name: "ExampleModule".to_string(),
            app_send,
            prop_send,
            registered_activities,
            registered_producers: Arc::new(Mutex::new(HashSet::new())),
            config: ExampleConfig::default(),
        })
    }

    fn get_name(&self) -> &'static str {
        NAME
    }
    // fn get_config(&self) -> &dyn ModuleConfig {
    //     &self.config
    // }

    fn get_registered_activities(&self) -> Rc<Mutex<ActivityMap>> {
        self.registered_activities.clone()
    }

    async fn register_activity(&self, activity: Rc<Mutex<DynamicActivity>>) {
        let mut reg = self.registered_activities.lock().await;
        reg.insert_activity(activity)
            .await
            .with_context(|| "failed to register activity")
            .unwrap();
    }
    async fn unregister_activity(&self, activity: &str) {
        self.registered_activities
            .lock()
            .await
            .map
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

    fn parse_config(&mut self, config: Value) -> Result<()> {
        self.config = config
            .into_rust()
            .with_context(|| "failed to parse config")
            .unwrap();
        Ok(())
    }

    fn init(&self) {
        let app_send = self.app_send.clone();
        let prop_send = self.prop_send.clone();
        glib::MainContext::default().spawn_local(async move {
            //create activity
            let activity = Rc::new(Mutex::new(Self::get_activity(
                prop_send,
                NAME,
                "exampleActivity1",
            )));

            //register activity and data producer
            app_send
                .send(UIServerCommand::AddActivity(
                    NAME.to_string(),
                    activity.clone(),
                ))
                .unwrap();
            app_send
                .send(UIServerCommand::AddProducer(
                    NAME.to_string(),
                    Self::producer as Producer,
                ))
                .unwrap();
        });
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ExampleModule {
    //TODO add reference to module and recieve messages from main
    #[allow(unused_variables)]
    fn producer(module: &dyn Module, rt: &Handle, _app_send: UnboundedSender<UIServerCommand>) {
        let module = cast_dyn_any!(module, ExampleModule).unwrap();
        //data producer
        let config: &ExampleConfig = &module.config;

        //TODO shouldn't be blocking locks, maybe execute async with glib::MainContext
        let act = module.registered_activities.blocking_lock();
        let mode = act
            .get_property_blocking("exampleActivity1", "mode")
            .unwrap();
        let label = act
            .get_property_blocking("exampleActivity1", "comp-label")
            .unwrap();
        let scrolling_text = act
            .get_property_blocking("exampleActivity1", "scrolling-label-text")
            .unwrap();
        let rolling_char = act
            .get_property_blocking("exampleActivity1", "rolling-char")
            .unwrap();
        // label.blocking_lock().set(config.string.clone()).unwrap();

        // let activity = Rc::new(Mutex::new(Self::get_activity(
        //     _prop_send.clone(),
        //     "exampleActivity2",
        // )));
        // _app_send
        //     .send(UIServerCommand::AddActivity(
        //         "ExampleModule".to_string(),
        //         activity,
        //     ))
        //     .unwrap();

        let config = config.clone();
        // debug!("starting task");
        rt.spawn(async move {
            // debug!("task started");
            mode.lock().await.set(ActivityMode::Minimal).unwrap();
            loop {
                rolling_char.lock().await.set('0').unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(config.duration)).await;
                rolling_char.lock().await.set('1').unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(config.duration)).await;
                rolling_char.lock().await.set('2').unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(config.duration)).await;
                rolling_char.lock().await.set('3').unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(config.duration)).await;
                rolling_char.lock().await.set('4').unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(config.duration)).await;

                scrolling_text
                    .lock()
                    .await
                    .set(
                        "Hello long text, very long text. Hello long text, very long text.    end"
                            .to_string(),
                    )
                    .unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(12000)).await;

                scrolling_text
                    .lock()
                    .await
                    .set("Hello shorterer e e e e text e.    end".to_string())
                    .unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(4000)).await;
                // mode.lock().await.set(ActivityMode::Minimal).unwrap();
                // tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

                // mode.lock().await.set(ActivityMode::Compact).unwrap();
                // tokio::time::sleep(tokio::time::Duration::from_millis(2500)).await;
                // let old_label_val;
                // {
                //     let label_val = label.lock().await;
                //     let str_val: &String = cast_dyn_any!(label_val.get(), String).unwrap();
                //     old_label_val = str_val.clone();
                // }

                // tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
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
        module: &str,
        name: &str,
    ) -> DynamicActivity {
        let mut activity = DynamicActivity::new(prop_send, module, name);

        //create activity widget
        let mut activity_widget = activity.get_activity_widget();
        Self::set_act_widget(&mut activity_widget);
        //get widgets
        // let background = Self::get_bg();
        let minimal = Self::get_minimal();
        let compact = Self::get_compact();
        let expanded = Self::get_expanded();
        let overlay = Self::get_overlay();

        //load widgets in the activity widget
        // activity_widget.add(&background);
        activity_widget.set_minimal_mode(&minimal);
        activity_widget.set_compact_mode(&compact);
        activity_widget.set_expanded_mode(&expanded);
        activity_widget.set_overlay_mode(&overlay);

        // activity_widget.connect_mode_notify(|f| {
        // let l = f.mode();
        // debug!("Changed mode: {:?}", l);
        // });

        // activity.set_activity_widget(activity_widget.clone());

        activity
            .add_dynamic_property("mode", ActivityMode::Minimal)
            .unwrap();
        activity
            .add_dynamic_property("comp-label", "compact".to_string())
            .unwrap();
        activity
            .add_dynamic_property("scrolling-label-text", "Hello, World".to_string())
            .unwrap();
        activity.add_dynamic_property("rolling-char", '0').unwrap();

        let minimal_cl = minimal.clone();
        activity
            .subscribe_to_property("scrolling-label-text", move |new_value| {
                let real_value = cast_dyn_any!(new_value, String).unwrap();
                log::debug!("text changed:{real_value}");
                minimal_cl
                    .clone()
                    .downcast::<gtk::Box>()
                    .unwrap()
                    .first_child()
                    .unwrap()
                    .downcast::<ScrollingLabel>()
                    .unwrap()
                    .label()
                    .set_text(real_value.as_str());
            })
            .unwrap();

        let mode = activity.get_property("mode").unwrap();

        let press_gesture = gtk::GestureClick::new();
        press_gesture.set_button(gdk::BUTTON_PRIMARY);

        let m1 = mode.clone();
        press_gesture.connect_released(move |_gest, _, _, _| {
            // debug!("primary");
            // gest.set_state(gtk::EventSequenceState::Claimed);
            let m1 = m1.clone();
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
        });

        activity_widget.add_controller(press_gesture);

        let m1 = mode.clone();
        let release_gesture = GestureClick::new();
        release_gesture.set_button(gdk::BUTTON_SECONDARY);
        release_gesture.connect_released(move |_gest, _, _, _| {
            // debug!("secondary");
            // gest.set_state(gtk::EventSequenceState::Claimed);
            let m1 = m1.clone();
            glib::MainContext::default().spawn_local(async move {
                let mode_g = m1.lock().await;
                let mode = *cast_dyn_any!(mode_g.get(), ActivityMode).unwrap();
                drop(mode_g);

                match mode {
                    ActivityMode::Minimal => {
                        log::warn!("Don't. It will crash and idk why");
                        // m1.lock().await.set(ActivityMode::Overlay).unwrap();
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
        });

        activity_widget.add_controller(release_gesture);

        //set mode when updated
        activity
            .subscribe_to_property("mode", move |new_value| {
                let real_value = cast_dyn_any!(new_value, ActivityMode).unwrap();
                activity_widget.set_mode(real_value);
            })
            .unwrap();

        let c1 = compact.clone();
        activity
            .subscribe_to_property("rolling-char", move |new_value| {
                let real_value = cast_dyn_any!(new_value, char).unwrap();
                let first_child = c1 //FIXME WTF is this, i need to change it, maybe with a macro
                    .clone()
                    .downcast::<gtk::Box>()
                    .unwrap()
                    .first_child()
                    .unwrap();

                let rolling_char_1 = first_child
                    .next_sibling()
                    .unwrap()
                    .downcast::<RollingChar>()
                    .unwrap();
                rolling_char_1.set_current_char(real_value);

                let rolling_char_2 = rolling_char_1
                    .next_sibling()
                    .unwrap()
                    .downcast::<RollingChar>()
                    .unwrap();
                rolling_char_2.set_current_char(real_value);
            })
            .unwrap();

        //set label when updated
        activity
            .subscribe_to_property("comp-label", move |new_value| {
                let real_value = cast_dyn_any!(new_value, String).unwrap();
                compact //FIXME WTF is this, i need to change it, maybe with a macro
                    .clone()
                    .downcast::<gtk::Box>()
                    .unwrap()
                    .first_child()
                    .unwrap()
                    .downcast::<gtk::Label>()
                    .unwrap()
                    .set_label(real_value);
            })
            .unwrap();

        activity
    }

    fn set_act_widget(_activity_widget: &mut ActivityWidget) {
        // activity_widget.set_vexpand(true);
        // activity_widget.set_hexpand(true);
        // activity_widget.set_valign(gtk::Align::Start);
        // activity_widget.set_halign(gtk::Align::Start);
        // activity_widget.set_transition_duration(2000, true).unwrap();
        // activity_widget.style_context().add_class("overlay");
    }

    fn get_minimal() -> gtk::Widget {
        let minimal = gtk::Box::builder()
            // .height_request(40)
            .width_request(240)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .vexpand(false)
            .hexpand(false)
            .overflow(gtk::Overflow::Hidden)
            .homogeneous(false)
            .build();

        let scroll_label = ScrollingLabel::new(Some("VDsdvzdfvzcxvvzxcvzcd"));
        scroll_label.set_hexpand(false);
        scroll_label.set_vexpand(false);
        scroll_label.set_valign(gtk::Align::Center);
        scroll_label.set_halign(gtk::Align::Start);
        // scroll_label.set_width_request(400);
        scroll_label.set_height_request(40);
        scroll_label.set_margin_start(20);
        scroll_label.set_margin_end(20);

        // let test_btn=gtk::Button::new();
        // test_btn.set_label("test");
        // test_btn.connect_clicked(|_btn|{
        //     log::info!("test");
        // });
        // let btn_gest=GestureClick::new();
        // btn_gest.set_button(gdk::BUTTON_PRIMARY);
        // btn_gest.connect_released(|gest,_,_,_|{
        //     gest.set_state(gtk::EventSequenceState::Claimed);
        //     log::info!("test");
        // });
        // test_btn.add_controller(btn_gest);

        // scroll_label.inner_label().set_margin_start(10);
        // scroll_label.inner_label().set_margin_end(30);

        minimal.append(&scroll_label);
        // minimal.append(&test_btn);
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

        compact.append(
            &Label::builder()
                .label("Compact")
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .build(),
        );

        let rn1 = RollingChar::new(None);
        rn1.set_valign(gtk::Align::Center);
        rn1.set_halign(gtk::Align::Center);
        compact.append(&rn1);

        let rn2 = RollingChar::new(None);
        rn2.set_valign(gtk::Align::Center);
        rn2.set_halign(gtk::Align::Center);
        compact.append(&rn2);

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

        expanded.append(
            &gtk::Label::builder()
                .label("Expanded label,\n Hello Hello")
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .hexpand(true)
                .build(),
        );
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

        expanded.append(
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
