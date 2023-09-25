use std::sync::Arc;

use gtk::prelude::*;
use tokio::{
    runtime::Runtime,
    sync::{mpsc::UnboundedSender, Mutex},
};

use crate::{
    app::ServerCommand,
    cast_dyn_prop,
    widgets::{
        activity_widget::{ActivityMode, ActivityWidget},
        dynamic_activity::DynamicActivity,
        dynamic_property::PropertyUpdate,
    },
};

pub struct ExampleModule {
    app_send: UnboundedSender<ServerCommand>,
}
impl ExampleModule {
    pub fn new(app_send: UnboundedSender<ServerCommand>) -> Self {
        Self {
            app_send
        }
    }

    pub fn init(&self){//TODO subdivide in phases

        //create ui channel
        let (prop_send, mut prop_recv) = tokio::sync::mpsc::unbounded_channel::<PropertyUpdate>();

        //TODO maybe move to server
        let app_send = self.app_send.clone();
        glib::MainContext::default().spawn_local(async move {
            //create activity
            let activity = Arc::new(Mutex::new(Self::get_activity(prop_send)));

            //register activity and data producer
            app_send.send(ServerCommand::AddActivity(activity.clone())).unwrap();
            app_send.send(ServerCommand::AddProducer(Self::producer)).unwrap();

            //start data consumer
            while let Some(res) = prop_recv.recv().await {
                match activity.lock().await.get_subscribers(&res.0) {
                    core::result::Result::Ok(subs) => {
                        for sub in subs {
                            sub(&*res.1);
                        }
                    }
                    Err(err) => eprintln!("{}", err),
                }
            }
        });
    }

    //TODO replace 'activities' with module context
    pub fn producer(activities: &[Arc<Mutex<DynamicActivity>>], rt: &Runtime) {
        //data producer
        let act = activities[0].blocking_lock();
        let mode = act.get_property("mode").unwrap();
        let label = act.get_property("comp-label").unwrap();

        rt.spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                mode.lock().await.set(ActivityMode::Minimal).unwrap();

                tokio::time::sleep(tokio::time::Duration::from_millis(4000)).await;
                mode.lock().await.set(ActivityMode::Compact).unwrap();
                let old_label_val;
                {
                    let label_val = label.lock().await;
                    let str_val: &String = (label_val.get() as &dyn std::any::Any)
                        .downcast_ref()
                        .unwrap();
                    old_label_val = str_val.clone();
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                label.lock().await.set("sdkjvksdv1".to_string()).unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                label.lock().await.set("fghn".to_string()).unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

                label.lock().await.set(old_label_val).unwrap();

                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                mode.lock().await.set(ActivityMode::Expanded).unwrap();

                tokio::time::sleep(tokio::time::Duration::from_millis(4000)).await;
                mode.lock().await.set(ActivityMode::Compact).unwrap();
            }
        });
    }

    fn get_activity(
        prop_send: tokio::sync::mpsc::UnboundedSender<PropertyUpdate>,
    ) -> DynamicActivity {
        let mut activity = DynamicActivity::new(prop_send, "exampleActivity");

        //create activity widget
        let activity_widget = Self::get_act_widget();
        //get widgets
        let background = Self::get_bg();
        let minimal = Self::get_minimal();
        let compact = Self::get_compact();
        let expanded = Self::get_expanded();

        //load widgets in the activity widget
        activity_widget.add(&background);
        activity_widget.set_minimal_mode(&minimal);
        activity_widget.set_compact_mode(&compact);
        activity_widget.set_expanded_mode(&expanded);

        activity_widget.connect_mode_notify(|f| {
            let l = f.mode();
            println!("Changed mode: {:?}", l);
        });
        activity.set_activity_widget(activity_widget.clone());

        activity
            .add_dynamic_property("mode", ActivityMode::Minimal)
            .unwrap();
        //set mode when updated
        activity
            .subscribe_to_property("mode", move |new_value| {
                let real_value = cast_dyn_prop!(new_value, ActivityMode).unwrap();
                activity_widget.set_mode(real_value);
            })
            .unwrap();

        activity
            .add_dynamic_property("comp-label", "compact".to_string())
            .unwrap();
        //set label when updated
        activity
            .subscribe_to_property("comp-label", move |new_value| {
                let real_value = cast_dyn_prop!(new_value, String).unwrap();
                compact
                    .clone()
                    .downcast::<gtk::Box>()
                    .unwrap()
                    .children()
                    .get(0)
                    .unwrap()
                    .clone()
                    .downcast::<gtk::Label>()
                    .unwrap()
                    .set_label(real_value);
            })
            .unwrap();

        activity
    }

    fn get_act_widget() -> ActivityWidget {
        let activity_widget = ActivityWidget::default();
        activity_widget.set_vexpand(false);
        activity_widget.set_hexpand(false);
        activity_widget.set_valign(gtk::Align::Start);
        activity_widget.set_halign(gtk::Align::Center);
        activity_widget.set_transition_duration(2000);
        activity_widget.style_context().add_class("overlay");
        activity_widget
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

        minimal.add(
            &gtk::Label::builder()
                .label("m")
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .hexpand(true)
                .build(),
        );
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
        compact.upcast()
    }

    fn get_expanded() -> gtk::Widget {
        let expanded = gtk::Box::builder()
            .height_request(100)
            .width_request(350)
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
}
