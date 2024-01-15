use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use anyhow::{Context, Ok, Result};
use async_trait::async_trait;
use gtk::{prelude::*, Widget};
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
    cast_dyn_any, cast_dyn_any_mut,
    graphics::activity_widget::widget::{ActivityMode, ActivityWidget},
};

//add to modules to be loaded
#[distributed_slice(MODULES)]
static EXAMPLE_MODULE: fn(UnboundedSender<UIServerCommand>, Option<Value>) -> Box<dyn Module> =
    MusicModule::new;

/// for now this is just used to test new code
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MusicConfig {
    //allowed_players: cider2, (?cider1, ?spotify...)
    enabled_player_override: Vec<String>,
}

impl ModuleConfig for MusicConfig {}

pub struct MusicModule {
    name: String,
    app_send: UnboundedSender<UIServerCommand>,
    prop_send: UnboundedSender<PropertyUpdate>,
    registered_activities: ActivityMap,
    registered_producers: Arc<Mutex<HashSet<Producer>>>,
    config: MusicConfig,
}

#[async_trait(?Send)]
impl Module for MusicModule {
    fn new(app_send: UnboundedSender<UIServerCommand>, config: Option<Value>) -> Box<dyn Module> {
        let conf = match config {
            Some(value) => value.into_rust().expect("failed to parse config"),
            None => MusicConfig::default(),
        };
        let registered_activities = Rc::new(Mutex::new(HashMap::<
            String,
            Rc<Mutex<DynamicActivity>>,
        >::new()));

        let prop_send = MusicModule::spawn_property_update_loop(&registered_activities);

        Box::new(Self {
            name: "MusicModule".to_string(),
            app_send,
            prop_send,
            registered_activities,
            registered_producers: Arc::new(Mutex::new(HashSet::new())),
            config: conf,
        })
    }

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
            let activity = Rc::new(Mutex::new(Self::get_activity(prop_send, "music-activity")));

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

fn update_config(config: &MusicConfig, activities: ActivityMap) {}

impl MusicModule {
    //TODO add reference to module and recieve messages from main
    #[allow(unused_variables)]
    fn producer(
        activities: ActivityMap,
        rt: &Handle,
        _app_send: UnboundedSender<UIServerCommand>,
        _prop_send: UnboundedSender<PropertyUpdate>,
        config: &dyn ModuleConfig,
        module: &mut dyn Module,
    ) {
        //data producer
        let config: &MusicConfig = cast_dyn_any!(config, MusicConfig).unwrap();
        let module: &mut MusicModule = cast_dyn_any_mut!(module, MusicModule).unwrap();
        update_config(config, activities.clone());
        let mode = activities
            .blocking_lock()
            .get("music-activity")
            .unwrap()
            .blocking_lock()
            .get_property("mode")
            .unwrap()
            .clone();
        // debug!("starting task");
        let config = config.clone();
        rt.spawn(async move {
            let prev_mode=*cast_dyn_any!(mode.lock().await.get(),ActivityMode).unwrap();
            if !matches!(prev_mode,ActivityMode::Expanded){
                mode.lock().await.set(ActivityMode::Expanded).unwrap();
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
        // let overlay = Self::get_overlay();

        //load widgets in the activity widget
        activity_widget.add(&background);
        activity_widget.set_minimal_mode(&minimal);
        activity_widget.set_compact_mode(&compact);
        activity_widget.set_expanded_mode(&expanded);
        // activity_widget.set_overlay_mode(&overlay);

        activity
            .add_dynamic_property("mode", ActivityMode::Minimal)
            .unwrap();

        //set mode when updated
        activity
            .subscribe_to_property("mode", move |new_value| {
                let real_value = cast_dyn_any!(new_value, ActivityMode).unwrap();
                activity_widget.set_mode(real_value);
            })
            .unwrap();

        activity
    }

    fn set_act_widget(activity_widget: &mut ActivityWidget) {
        activity_widget.set_vexpand(false);
        activity_widget.set_hexpand(false);
        activity_widget.set_valign(gtk::Align::Start);
        activity_widget.set_halign(gtk::Align::Center);
    }

    fn get_bg() -> gtk::Widget {
        let background = gtk::Label::builder()
            .label("")
            .halign(gtk::Align::Start)
            .valign(gtk::Align::Start)
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
        let height = 300;
        let width = 450;
        let v_container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .height_request(height)
            .width_request(width)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .vexpand(false)
            .hexpand(false)
            .build();

        let info_container = info_container(width as f32, height as f32 * 0.45);
        let progress_container = progress_container(width as f32, height as f32 * 0.15);
        let controls_container = controls_container(width as f32, height as f32 * 0.40);

        v_container.add(&info_container);
        v_container.add(&progress_container);
        v_container.add(&controls_container);

        let expanded = gtk::EventBox::builder()
            .height_request(height)
            .width_request(width)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .vexpand(false)
            .hexpand(false)
            .above_child(false)
            .child(&v_container)
            .build();
        expanded.upcast()
    }

    // fn get_overlay() -> gtk::Widget {
    //     let overlay = gtk::Box::builder()
    //         .height_request(1080)
    //         .width_request(1920)
    //         .valign(gtk::Align::Center)
    //         .halign(gtk::Align::Center)
    //         .vexpand(false)
    //         .hexpand(false)
    //         .build();
    //     overlay.add(
    //         &gtk::Label::builder()
    //             .label("Overlay label,\n Hello Hello \n Hello Hello")
    //             .halign(gtk::Align::Center)
    //             .valign(gtk::Align::Center)
    //             .hexpand(true)
    //             .build(),
    //     );
    //     overlay.upcast()
    // }
}

fn info_container(width: f32, height: f32) -> Widget {
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .height_request(height as i32)
        .width_request(width as i32)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .build();
    container.style_context().add_class("info");

    let album_art_size=height.min(width * 0.3);
    let album_art = gtk::Image::builder()
        // .file("/home/david/Pictures/Music_not_playing.svg")
        .width_request(album_art_size as i32)
        .build();
    let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_file_at_size(
        "/home/david/Pictures/Music_not_playing.svg",
        (album_art_size*0.7) as i32,
        (album_art_size*0.7) as i32,
        ).expect("failed to load image");
    album_art.set_from_pixbuf(Some(&pixbuf));
    album_art.style_context().add_class("album-art");
    
    let music_info_container=gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .height_request(height as i32)
        .width_request((width*0.50) as i32)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Start)
        .vexpand(false)
        .hexpand(false)
        .homogeneous(true)
        .build();
    music_info_container.style_context().add_class("info-names1");
    let music_info_container2=gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Start)
        .vexpand(false)
        .hexpand(false)
        .build();
    music_info_container2.style_context().add_class("info-names2");
    let song_name=gtk::Label::builder() //TODO replace with scrollable label
        .label("Song name")
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Center)
        .wrap(false)
        .hexpand(true)
        .build();
    song_name.style_context().add_class("song-name");
    let artist_name=gtk::Label::builder() //TODO replace with scrollable label
        .label("Artist name")
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Center)
        .wrap(false)
        .hexpand(true)
        .build();
    artist_name.style_context().add_class("artist-name");
    music_info_container2.add(&song_name);
    music_info_container2.add(&artist_name);
    music_info_container.add(&music_info_container2);

    let visualizer_size=height.min(width * 0.2); //TODO replace with actual visualizer
    let visualizer = gtk::Image::builder()
        // .height_request((visualizer_size*0.8) as i32)
        .width_request(visualizer_size as i32)
        .build();
    let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_file_at_size(
        "/home/david/Pictures/visualizer_tmp.jpeg",
        (visualizer_size*0.8) as i32,
        (visualizer_size*0.8) as i32,
        ).expect("failed to load image");
    visualizer.set_from_pixbuf(Some(&pixbuf));
    visualizer.style_context().add_class("visualizer");

    container.add(&album_art);
    container.add(&music_info_container);
    container.add(&visualizer);

    container.into()
}
fn progress_container(width: f32, height: f32) -> Widget {
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .height_request((height) as i32)
        .width_request(width as i32)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .build();
    container.style_context().add_class("progress");

    let elapsed=gtk::Label::builder()
        .label("0:00")
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .width_request((width*0.15) as i32)
        .build();
    elapsed.style_context().add_class("elapsed-time");
    let progress_bar=gtk::Scale::builder()
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .width_request((width*0.7) as i32)
        .build();
    progress_bar.set_range(0.0, 1.0);
    progress_bar.set_draw_value(false);
    progress_bar.set_increments((1.0/(width*0.7)).into(), 0.1);
    progress_bar.style_context().add_class("progress-bar");
    let remaining=gtk::Label::builder()
        .label("-3:42")
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .width_request((width*0.15) as i32)
        .build();
    remaining.style_context().add_class("remaining-time");

    container.add(&elapsed);
    container.add(&progress_bar);
    container.add(&remaining);

    container.into()
}
fn controls_container(width: f32, height: f32) -> Widget {
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .height_request((height) as i32)
        .width_request(width as i32)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .homogeneous(true)
        .build();

    container.style_context().add_class("controls");

    let shuffle=gtk::Button::builder()
        .label("Sh")
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .height_request((height.min(width*0.15)) as i32)
        .width_request((width*0.15) as i32)
        .build();
    shuffle.style_context().add_class("shuffle");
    let previous=gtk::Button::builder()
        .label("Pr")
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .height_request((height.min(width*0.2)) as i32)
        .width_request((width*0.2) as i32)
        .build();
    previous.style_context().add_class("previous");
    let play_pause=gtk::Button::builder()
        .label("Pl")
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .height_request((height.min(width*0.2)) as i32)
        .width_request((width*0.2) as i32)
        .build();
    play_pause.style_context().add_class("play-pause");
    let next=gtk::Button::builder()
        .label("Nx")
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .height_request((height.min(width*0.2)) as i32)
        .width_request((width*0.2) as i32)
        .build();
    next.style_context().add_class("next");
    let repeat=gtk::Button::builder()
        .label("Rp")
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .height_request((height.min(width*0.15)) as i32)
        .width_request((width*0.15) as i32)
        .build();

    container.add(&shuffle);
    container.add(&previous);
    container.add(&play_pause);
    container.add(&next);
    container.add(&repeat);

    container.into()
}
