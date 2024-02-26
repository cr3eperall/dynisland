use std::{collections::HashSet, rc::Rc, sync::Arc, vec};

use anyhow::{Context, Ok, Result};
use gtk::{prelude::*, GestureClick, Label};
use ron::Value;
use serde::{Deserialize, Serialize};
use tokio::{
    runtime::Handle,
    sync::{mpsc::UnboundedSender, Mutex},
};

use dynisland_core::{
    base_module::{
        ActivityMap, DynamicActivity, Module, Producer, PropertyUpdate
    },
    cast_dyn_any,
    graphics::{
        activity_widget::{imp::ActivityMode, ActivityWidget},
        widgets::{rolling_char::RollingChar, scrolling_label::ScrollingLabel},
    }, module_abi::{ActivityIdentifier, UIServerCommand},
};

use super::NAME;

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
    pub registered_activities: Rc<Mutex<ActivityMap>>,
    registered_producers: Arc<Mutex<HashSet<Producer>>>,
    pub producers_handle: Handle,
    pub producers_shutdown: tokio::sync::mpsc::Sender<()>,
    config: ExampleConfig,
}

fn register_activity(registered_activities: Rc<Mutex<ActivityMap>>, app_send: &UnboundedSender<UIServerCommand>, activity: DynamicActivity) {
    
    let widget=activity.get_activity_widget();
    let id=activity.get_identifier();
    let activity= Rc::new(Mutex::new(activity));
    
    app_send
        .send(UIServerCommand::AddActivity(
            id,
            widget.into(),
        ))
        .unwrap();
    let mut reg = registered_activities.blocking_lock();
    reg.insert_activity(activity)
        .with_context(|| "failed to register activity")
        .unwrap();
}
fn _unregister_activity(registered_activities: Rc<Mutex<ActivityMap>>, app_send: &UnboundedSender<UIServerCommand>, activity_name: &str) {
    app_send
        .send(UIServerCommand::RemoveActivity(ActivityIdentifier::new(NAME, activity_name)))
        .unwrap();
    
    registered_activities
        .blocking_lock()
        .map
        .remove(activity_name)
        .expect("activity isn't registered");
}

pub fn get_new_tokio_rt() -> (Handle, tokio::sync::mpsc::Sender<()>) {
    let (rt_send, rt_recv) =
        tokio::sync::oneshot::channel::<(Handle, tokio::sync::mpsc::Sender<()>)>();
    let (shutdown_send, mut shutdown_recv) = tokio::sync::mpsc::channel::<()>(1);
    std::thread::Builder::new()
        .name("dyn-producers".to_string())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("idk tokio rt failed");
            let handle = rt.handle();
            rt_send
                .send((handle.clone(), shutdown_send))
                .expect("failed to send rt");
            rt.block_on(async { shutdown_recv.recv().await }); //keep thread alive
        })
        .expect("failed to spawn new trhread");

    rt_recv.blocking_recv().expect("failed to receive rt")
}

impl Module for ExampleModule {
    fn new(app_send: UnboundedSender<UIServerCommand> ) -> Box<dyn Module> {
        let registered_activities = Rc::new(Mutex::new(ActivityMap::new()));

        let prop_send = ExampleModule::spawn_property_update_loop(&registered_activities);
        let (hdl, shutdown) = get_new_tokio_rt();
        Box::new(Self {
            // name: "ExampleModule".to_string(),
            app_send,
            prop_send,
            registered_activities,
            registered_producers: Arc::new(Mutex::new(HashSet::new())),
            producers_handle: hdl,
            producers_shutdown: shutdown,
            config: ExampleConfig::default(),
        })
    }

    fn restart_producers(&mut self) {
        self.restart_producer_rt();
    }

    fn get_registered_producers(&self) -> Arc<Mutex<HashSet<Producer>>> {
        self.registered_producers.clone()
    }

    fn update_config(&mut self, config: Value) -> Result<()> {
        self.config = config
            .into_rust()
            .with_context(|| "failed to parse config")
            .unwrap();
        Ok(())
    }

    fn init(&self) {
        let app_send = self.app_send.clone();
        let prop_send = self.prop_send.clone();
        let registered_activities = self.registered_activities.clone();
        // glib::MainContext::default().spawn_local(async move { //FIXME this assumes that init is called from the main thread, now it is but it may change
            
            //create activity
            let act = Self::get_activity(
                prop_send,
                NAME,
                "exampleActivity1",
            );

            //register activity and data producer
            register_activity(registered_activities, &app_send, act);
        // });
        self.register_producer(Self::producer);
    }
}

impl ExampleModule {

    fn register_producer(&self, producer: Producer) {
        producer(
            self as &dyn Module,
            &self.producers_handle,
            self.app_send.clone(),
        );
        
        // app_send
        //     .send(UIServerCommand::AddProducer(
        //         NAME.to_string(),
        //         producer as Producer,
        //     ))
        //     .unwrap();
        self.registered_producers.blocking_lock().insert(producer);
    }

    fn restart_producer_rt(&mut self) {
        self.producers_shutdown
            .blocking_send(())
            .expect("failed to shutdown old producer runtime"); //stop current producers_runtime
        let (handle, shutdown) = get_new_tokio_rt(); //start new producers_runtime
        self.producers_handle = handle;
        self.producers_shutdown = shutdown;
        //restart producers
        for producer in self.get_registered_producers().blocking_lock().iter() {
            producer(
                self as &dyn Module,
                &self.producers_handle,
                self.app_send.clone(),
            )
        }
    }

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
