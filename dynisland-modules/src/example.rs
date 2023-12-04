use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::{Context, Ok, Result};
use async_trait::async_trait;
use gtk::prelude::*;
use linkme::distributed_slice;
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
    widgets::activity_widget::{ActivityMode, ActivityWidget},
};

#[distributed_slice(MODULES)]
static EXAMPLE_MODULE: fn(UnboundedSender<UIServerCommand>, Option<Value>) -> Box<dyn Module> =
    ExampleModule::new;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExampleConfig {
    pub int: i32,
    pub string: String,
    pub vec: Vec<String>,
}
impl ModuleConfig for ExampleConfig {}
impl Default for ExampleConfig {
    fn default() -> Self {
        Self {
            int: 0,
            string: String::from("Example1"),
            vec: vec![String::from("Example2"), String::from("Example3")],
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
        let registered_activities = Arc::new(Mutex::new(HashMap::<
            String,
            Arc<Mutex<DynamicActivity>>,
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
    //                             // eprintln!("{}", err)
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
    //                                 // eprintln!("{}", err)
    //                             }
    //                         }
    //                     }
    //                     None => {
    //                         eprintln!("activity {} not found on ExampleModule", res.activity_id);
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

    fn get_registered_activities(
        &self,
    ) -> Arc<Mutex<HashMap<String, Arc<Mutex<DynamicActivity>>>>> {
        self.registered_activities.clone()
    }

    async fn register_activity(&self, activity: Arc<Mutex<DynamicActivity>>) {
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
        //TODO subdivide in phases

        //TODO maybe move to server
        let app_send = self.app_send.clone();
        let name = self.name.clone();
        let prop_send = self.prop_send.clone();
        glib::MainContext::default().spawn_local(async move {
            //create activity
            let activity = Arc::new(Mutex::new(Self::get_activity(
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

impl ExampleModule {
    //TODO replace 'activities' with module context
    fn producer(
        activities: ActivityMap,
        rt: &Handle,
        _app_send: UnboundedSender<UIServerCommand>,
        _prop_send: UnboundedSender<PropertyUpdate>,
        config: &dyn ModuleConfig,
    ) {
        //data producer
        let config: &ExampleConfig = cast_dyn_any!(config, ExampleConfig).unwrap();
        //TODO shouldn't be blocking locks, maybe execute async with glib::MainContext
        let act = activities.blocking_lock();
        let _mode = act
            .get("exampleActivity1")
            .unwrap()
            .blocking_lock()
            .get_property("mode")
            .unwrap();
        let label = act
            .get("exampleActivity1")
            .unwrap()
            .blocking_lock()
            .get_property("comp-label")
            .unwrap();
        label.blocking_lock().set(config.string.clone()).unwrap();
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

        // println!("starting task");
        rt.spawn(async move {
            // println!("task started");
            // mode.lock().await.set(ActivityMode::Minimal).unwrap();
            // loop {
                // mode.lock().await.set(ActivityMode::Minimal).unwrap();
                // tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
                // println!("mode updated");

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
                // mode.lock().await.set(ActivityMode::Expanded).unwrap();
                // tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
                // mode.lock().await.set(ActivityMode::Compact).unwrap();
                // tokio::time::sleep(tokio::time::Duration::from_millis(2500)).await;
            // }
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
        //     let l = f.mode();
        //     println!("Changed mode: {:?}", l);
        // });
        activity.set_activity_widget(activity_widget.clone());

        activity
            .add_dynamic_property("mode", ActivityMode::Minimal)
            .unwrap();
        activity
            .add_dynamic_property("comp-label", "compact".to_string())
            .unwrap();

        let mode = activity.get_property("mode").unwrap();

        minimal.add_events(gdk::EventMask::BUTTON_RELEASE_MASK);
        let m1 = mode.clone();
        minimal.connect_button_release_event(move |_wid, ev| {
            if let gdk::EventType::ButtonRelease = ev.event_type() {
                let m1 = m1.clone();
                glib::MainContext::default().spawn_local(async move {
                    m1.lock().await.set(ActivityMode::Compact).unwrap();
                });
            }
            glib::Propagation::Proceed
        });

        compact.add_events(gdk::EventMask::BUTTON_RELEASE_MASK);
        let m1 = mode.clone();
        compact.connect_button_release_event(move |_wid, ev| {
            if let gdk::EventType::ButtonRelease = ev.event_type() {
                let m1 = m1.clone();
                glib::MainContext::default().spawn_local(async move {
                    m1.lock().await.set(ActivityMode::Minimal).unwrap();
                });
            }
            glib::Propagation::Proceed
        });

        // let prop=activity.get_property("comp-label").unwrap();
        // compact.connect_enter_notify_event(move |m1, m2|{
        //     println!("{m2:?}");
        //     prop.blocking_lock().set(format!("{:?}",m2.coords().unwrap())).unwrap();
        //     glib::Propagation::Proceed
        // });
        //set mode when updated
        activity
            .subscribe_to_property("mode", move |new_value| {
                let real_value = cast_dyn_any!(new_value, ActivityMode).unwrap();
                activity_widget.set_mode(real_value);
            })
            .unwrap();

        //set label when updated
        activity
            .subscribe_to_property("comp-label", move |new_value| {
                let real_value = cast_dyn_any!(new_value, String).unwrap();
                compact
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
                    .downcast::<gtk::Label>()
                    .unwrap()
                    .set_label(real_value);
            })
            .unwrap();

        activity
    }

    fn set_act_widget(activity_widget: &mut ActivityWidget) {
        activity_widget.set_vexpand(false);
        activity_widget.set_hexpand(false);
        activity_widget.set_valign(gtk::Align::Start);
        activity_widget.set_halign(gtk::Align::Center);
        // activity_widget.set_transition_duration(2000, true).unwrap();
        activity_widget.style_context().add_class("overlay");
    }

    fn get_bg() -> gtk::Widget {
        let background = gtk::Label::builder()
            .label("")
            .halign(gtk::Align::Start)
            .valign(gtk::Align::Start)
            .build();
        background.upcast()
    }

    fn get_minimal() -> gtk::Widget {
        let minimal = gtk::Box::builder()
            .height_request(40)
            .width_request(50)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .vexpand(false)
            .hexpand(false)
            .build();

        let btn = gtk::Label::builder()
            .label("m")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .build();

        minimal.add(&btn);
        let minimal = gtk::EventBox::builder()
            .height_request(40)
            .width_request(50)
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
            .height_request(40)
            .width_request(180)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .vexpand(false)
            .hexpand(false)
            .build();

        compact.add(
            &gtk::Label::builder()
                .label("compact")
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .hexpand(true)
                .build(),
        );
        let compact = gtk::EventBox::builder()
            .height_request(40)
            .width_request(180)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .vexpand(false)
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
//         //TODO subdivide in phases

//         //create ui channel
//         let (prop_send, mut prop_recv) = tokio::sync::mpsc::unbounded_channel::<PropertyUpdate>();

//         //TODO maybe move to server
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
//                     Err(err) => eprintln!("{}", err),
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
//     //TODO replace 'activities' with module context
//     fn producer(
//         activities: ActivityMap,
//         rt: &Handle,
//         _app_send: UnboundedSender<UIServerCommand>,
//         config: &dyn ModuleConfig,
//     ) {
//         //data producer
//         let _config: &ExampleConfig = cast_dyn_any!(config, ExampleConfig).unwrap();
//         //TODO shouldn't be blocking locks, maybe execute async with glib::MainContext
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
//         //     println!("Changed mode: {:?}", l);
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
