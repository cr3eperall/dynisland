use std::{
    f64::consts::PI,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use glib::subclass::types::ObjectSubclassExt;
use gtk::{prelude::*, subclass::widget::WidgetImpl};
use log::{debug, error};

use super::{
    util::{get_final_widget_size, get_max_preferred_size},
    widget::ActivityWidgetPriv,
    CLIP_CORRECTIVE_FACTOR, FILTER_BACKEND, TRANSLATE_CORRECTIVE_FACTOR,
};

impl WidgetImpl for ActivityWidgetPriv {
    fn preferred_width_for_height(&self, height: i32) -> (i32, i32) {
        let min_height = self.local_css_context.borrow().get_minimal_height();
        match &*self.background_widget.borrow() {
            Some(content) if content.is_visible() => {
                get_max_preferred_size(content.preferred_width_for_height(height), (height, height))
            }
            _ => (min_height, min_height),
        }
    }
    fn preferred_height_for_width(&self, width: i32) -> (i32, i32) {
        let min_height = self.local_css_context.borrow().get_minimal_height();
        match &*self.background_widget.borrow() {
            Some(content) if content.is_visible() => {
                get_max_preferred_size(content.preferred_height_for_width(width), (0, width))
            }
            _ => (min_height, min_height),
        }
    }

    fn preferred_height(&self) -> (i32, i32) {
        let min_height = self.local_css_context.borrow().get_minimal_height();
        match &*self.background_widget.borrow() {
            Some(content) if content.is_visible() => content.preferred_height(),
            _ => (min_height, min_height),
        }
    }

    fn preferred_width(&self) -> (i32, i32) {
        let min_height = self.local_css_context.borrow().get_minimal_height();
        match &*self.background_widget.borrow() {
            Some(content) if content.is_visible() => content.preferred_width(),
            _ => (min_height, min_height),
        }
    }

    fn size_allocate(&self, allocation: &gdk::Rectangle) {
        // trace!("activity allocate: ({}, {})", allocation.width(), allocation.height());

        if let Some(content) = &*self.background_widget.borrow() {
            content.size_allocate(allocation);
            self.obj().set_allocation(&content.allocation());
        } else {
            self.obj().set_allocation(allocation);
        }

        if let Some(content) = &*self.minimal_mode_widget.borrow() {
            let allocation = self.get_child_aligned_allocation(content);
            content.size_allocate(&allocation);
        }
        if let Some(content) = &*self.compact_mode_widget.borrow() {
            let allocation = self.get_child_aligned_allocation(content);
            content.size_allocate(&allocation);
        }
        if let Some(content) = &*self.expanded_mode_widget.borrow() {
            let allocation = self.get_child_aligned_allocation(content);
            content.size_allocate(&allocation);
        }
        if let Some(content) = &*self.overlay_mode_widget.borrow() {
            let allocation = self.get_child_aligned_allocation(content);
            content.size_allocate(&allocation);
        }

        if let Some(widget) = &*self.get_mode_widget(*self.mode.borrow()).borrow() {
            let (width, height) = get_final_widget_size(
                widget,
                *self.mode.borrow(),
                self.local_css_context.borrow().get_minimal_height(),
            );
            self.local_css_context
                .borrow_mut()
                .set_size((width, height))
                .expect("failed to set activity size");
        }
        // trace!("css_size: {:?}",self.local_css_context.borrow().get_size());
    }

    fn draw(&self, cr: &gdk::cairo::Context) -> glib::Propagation {
        // FIXME probably need to fix margins like in scrolling_label
        let mut logs: Vec<String> = vec![];
        let start = Instant::now();
        let mut time = Instant::now();

        let res: Result<()> = try {
            cr.save()?;
            cr.move_to(
                self.obj().allocation().x() as f64,
                self.obj().allocation().y() as f64,
            );
            let self_w = self.obj().allocation().width() as f64;
            let self_h = self.obj().allocation().height() as f64;
            let border_radius: i32 = self
                .obj()
                .style_context()
                .style_property_for_state("border-radius", self.obj().state_flags())
                .get()?;
            let border_radius = border_radius as f64;
            let radius = f64::min(border_radius, f64::min(self_w / 2.0, self_h / 2.0));

            self.local_css_context
                .borrow_mut()
                .set_border_radius(radius as i32)
                .expect("failed to set activity border-radius");

            //draw background
            gtk::render_background(&self.obj().style_context(), cr, 0.0, 0.0, self_w, self_h);

            // //draw background widget
            // if let Some(bg_widget) = &*self.background_widget.borrow() {
            //     self.obj().propagate_draw(bg_widget, cr);
            // }

            //setup clip
            begin_draw_scaled_clip(cr, (self_w, self_h), (self_w, self_h), (1.0, 1.0), radius);

            logs.push(format!("bg + clip setup {:?}", time.elapsed()));
            time = Instant::now();

            //draw active mode widget
            let widget_to_render = self.get_mode_widget(*self.mode.borrow());

            //animate blur, opacity and stretch if during transition
            if self.transition_manager.borrow_mut().has_running() {
                // trace!("{}, start: {:?}, dur: {:?}",progress, self.transition.borrow().start_time.elapsed(), self.transition.borrow().duration);
                let last_widget_to_render = self.get_mode_widget(*self.last_mode.borrow());

                let prev_size = if let Some(widget) = &*last_widget_to_render.borrow() {
                    get_final_widget_size(
                        widget,
                        *self.last_mode.borrow(),
                        self.local_css_context.borrow().get_minimal_height(),
                    )
                } else {
                    (0, 0)
                };
                let next_size = if let Some(widget) = &*widget_to_render.borrow() {
                    get_final_widget_size(
                        widget,
                        *self.mode.borrow(),
                        self.local_css_context.borrow().get_minimal_height(),
                    )
                } else {
                    (0, 0)
                };

                let mut tmp_surface_1 = gtk::cairo::ImageSurface::create(
                    gdk::cairo::Format::ARgb32,
                    self_w as i32,
                    self_h as i32,
                )
                .with_context(|| "failed to create new imagesurface")?;
                //PREV
                if let Some(widget) = &*last_widget_to_render.borrow() {
                    let tmp_cr = gdk::cairo::Context::new(&tmp_surface_1)
                        .with_context(|| "failed to retrieve context from tmp surface")?;

                    let sx = self
                        .transition_manager
                        .borrow_mut()
                        .get_value(&(self.last_mode.borrow().to_string() + "-stretch-x"))
                        .unwrap();
                    let sy = self
                        .transition_manager
                        .borrow_mut()
                        .get_value(&(self.last_mode.borrow().to_string() + "-stretch-y"))
                        .unwrap();
                    // debug!("PREV: sx: {}, sy: {}", sx, sy);
                    // let (mut sx, mut sy) = (
                    //     self_w / prev_size.0 as f64,
                    //     self_h / prev_size.1 as f64,
                    // );
                    // let scale_prog = self.timing_functions(
                    //     progress,
                    //     if bigger {
                    //         TimingFunction::BiggerStretch
                    //     } else {
                    //         TimingFunction::SmallerStretch
                    //     },
                    // );

                    // sx = (1.0 - scale_prog) + sx * scale_prog; // 0->1 | 1-> +sx >1 | 0.5-> 0.5+sx/2=(1+sx)/2 >1
                    // sy = (1.0 - scale_prog) + sy * scale_prog;

                    //setup clip
                    let radius = f64::min(
                        border_radius,
                        f64::min(
                            (prev_size.0 as f64 * sx) / 2.0,
                            (prev_size.1 as f64 * sy) / 2.0,
                        ),
                    );

                    begin_draw_scaled_clip(
                        &tmp_cr,
                        (
                            self_w - CLIP_CORRECTIVE_FACTOR,
                            self_h - CLIP_CORRECTIVE_FACTOR,
                        ),
                        (
                            prev_size.0 as f64 + CLIP_CORRECTIVE_FACTOR,
                            prev_size.1 as f64 + CLIP_CORRECTIVE_FACTOR,
                        ),
                        (sx, sy),
                        radius,
                    );

                    //scale and center
                    tmp_cr.scale(sx, sy);

                    tmp_cr.translate(
                        TRANSLATE_CORRECTIVE_FACTOR
                            + (self_w - prev_size.0 as f64 * sx) / (2.0 * sx),
                        TRANSLATE_CORRECTIVE_FACTOR
                            + (self_h - prev_size.1 as f64 * sy) / (2.0 * sy),
                    );

                    // tmp_cr.translate(
                    //     //V
                    //     TRANSLATE_CORRECTIVE_FACTOR-(self_w - prev_size.0 as f64) / 2.0
                    //         + (self_w - prev_size.0 as f64 * sx) / (2.0 * sx),
                    //     TRANSLATE_CORRECTIVE_FACTOR-(self_h - prev_size.1 as f64) / 2.0
                    //         + (self_h - prev_size.1 as f64 * sy) / (2.0 * sy),
                    // );

                    self.obj().propagate_draw(widget, &tmp_cr);

                    tmp_cr.reset_clip();

                    logs.push(format!(
                        "prev_widget draw + clip + scale {:?}",
                        time.elapsed()
                    ));
                    time = Instant::now();
                    drop(tmp_cr);

                    // crate::filters::filter::apply_blur(
                    //     &mut tmp_surface_1,
                    //     ActivityWidgetPriv::timing_functions(progress, TimingFunction::PrevBlur)
                    //         * RAD,
                    //     FILTER_BACKEND,
                    // )
                    // .with_context(|| "failed to apply blur to tmp surface")?;

                    // logs.push(format!("prev blur processed {:?}", time.elapsed()));
                    // time = Instant::now();

                    // cr.set_source_surface(&tmp_surface_1, 0.0, 0.0)
                    //     .with_context(|| "failed to set source surface")?;

                    // cr.paint_with_alpha(ActivityWidgetPriv::timing_functions(
                    //     progress,
                    //     TimingFunction::PrevOpacity,
                    // ) as f64)
                    //     .with_context(|| "failed to paint surface to context")?;

                    // logs.push(format!("prev blur written to surface {:?}", time.elapsed()));
                    // time = Instant::now();
                }

                let mut tmp_surface_2 = gtk::cairo::ImageSurface::create(
                    gdk::cairo::Format::ARgb32,
                    self_w as i32,
                    self_h as i32,
                )
                .with_context(|| "failed to create new imagesurface")?;
                //NEXT
                if let Some(widget) = &*widget_to_render.borrow() {
                    let tmp_cr = gdk::cairo::Context::new(&tmp_surface_2)
                        .with_context(|| "failed to retrieve context from tmp surface")?;

                    let sx = self
                        .transition_manager
                        .borrow_mut()
                        .get_value(&(self.mode.borrow().to_string() + "-stretch-x"))
                        .unwrap();
                    let sy = self
                        .transition_manager
                        .borrow_mut()
                        .get_value(&(self.mode.borrow().to_string() + "-stretch-y"))
                        .unwrap();
                    // debug!("NEXT: sx: {}, sy: {}", sx, sy);
                    // let (mut sx, mut sy) = (
                    //     self_w / next_size.0 as f64,
                    //     self_h / next_size.1 as f64,
                    // );

                    // let scale_prog =self.timing_functions(
                    //     1.0 - progress,
                    //     if bigger {
                    //         TimingFunction::SmallerStretch
                    //     } else {
                    //         TimingFunction::BiggerStretch
                    //     },
                    // ) as f64;

                    // sx = (1.0 - scale_prog) + sx * scale_prog; // 0->1 | 1-> +sx >1 | 0.5-> 0.5+sx/2=(1+sx)/2 >1
                    // sy = (1.0 - scale_prog) + sy * scale_prog;

                    //setup clip
                    let radius = f64::min(
                        border_radius,
                        f64::min(
                            (next_size.0 as f64 * sx) / 2.0,
                            (next_size.1 as f64 * sx) / 2.0,
                        ),
                    );

                    begin_draw_scaled_clip(
                        &tmp_cr,
                        (
                            self_w - CLIP_CORRECTIVE_FACTOR,
                            self_h - CLIP_CORRECTIVE_FACTOR,
                        ),
                        (
                            next_size.0 as f64 + CLIP_CORRECTIVE_FACTOR,
                            next_size.1 as f64 + CLIP_CORRECTIVE_FACTOR,
                        ),
                        (sx, sy),
                        radius,
                    );

                    //scale and center

                    tmp_cr.scale(sx, sy);

                    tmp_cr.translate(
                        TRANSLATE_CORRECTIVE_FACTOR
                            + (self_w - next_size.0 as f64 * sx) / (2.0 * sx),
                        TRANSLATE_CORRECTIVE_FACTOR
                            + (self_h - next_size.1 as f64 * sy) / (2.0 * sy),
                    );

                    // tmp_cr.translate(
                    //     //V
                    //     TRANSLATE_CORRECTIVE_FACTOR-(self_w - next_size.0 as f64) / 2.0
                    //         + (self_w - next_size.0 as f64 * sx) / (2.0 * sx),
                    //     TRANSLATE_CORRECTIVE_FACTOR-(self_h - next_size.1 as f64) / 2.0
                    //         + (self_h - next_size.1 as f64 * sy) / (2.0 * sy),
                    // );

                    self.obj().propagate_draw(widget, &tmp_cr);

                    tmp_cr.reset_clip();

                    logs.push(format!(
                        "next_widget draw + clip + scale {:?}",
                        time.elapsed()
                    ));
                    time = Instant::now();

                    drop(tmp_cr);

                    // crate::filters::filter::apply_blur(
                    //     &mut tmp_surface_2,
                    //     ActivityWidgetPriv::timing_functions(progress, TimingFunction::NextBlur)
                    //         * RAD,
                    //     FILTER_BACKEND,
                    // )
                    // .with_context(|| "failed to apply blur to tmp surface")?;

                    // logs.push(format!("next blur processed {:?}", time.elapsed()));
                    // time = Instant::now();

                    // cr.set_source_surface(&tmp_surface_2, 0.0, 0.0)
                    //     .with_context(|| "failed to set source surface")?;

                    // cr.paint_with_alpha(ActivityWidgetPriv::timing_functions(
                    //     progress,
                    //     TimingFunction::NextOpacity,
                    // ) as f64)
                    //     .with_context(|| "failed to paint surface to context")?;

                    // logs.push(format!("next blur written to surface {:?}", time.elapsed()));
                    // time = Instant::now();
                }

                let last_blur = self
                    .transition_manager
                    .borrow_mut()
                    .get_value(&(self.last_mode.borrow().to_string() + "-blur"))
                    .unwrap();
                let next_blur = self
                    .transition_manager
                    .borrow_mut()
                    .get_value(&(self.mode.borrow().to_string() + "-blur"))
                    .unwrap();

                let last_opacity = self
                    .transition_manager
                    .borrow_mut()
                    .get_value(&(self.last_mode.borrow().to_string() + "-opacity"))
                    .unwrap();
                let next_opacity = self
                    .transition_manager
                    .borrow_mut()
                    .get_value(&(self.mode.borrow().to_string() + "-opacity"))
                    .unwrap();

                // let mut orig_surface = cr.group_target();

                // debug!("last_blur: {}, next_blur: {}, last_opacity: {}, next_opacity: {}\n", last_blur, next_blur, last_opacity, next_opacity);
                crate::filters::filter::apply_blur_and_merge_opacity_dual(
                    // &mut orig_surface,
                    &mut tmp_surface_1,
                    &mut tmp_surface_2,
                    last_blur as f32,
                    next_blur as f32,
                    last_opacity as f32,
                    next_opacity as f32,
                    FILTER_BACKEND,
                )
                .with_context(|| "failed to apply double blur + merge to tmp surface")?;

                logs.push(format!("double blur processed {:?}", time.elapsed()));
                time = Instant::now();

                cr.set_source_surface(&tmp_surface_1, 0.0, 0.0)
                    .with_context(|| "failed to set source surface")?;

                cr.paint()
                    .with_context(|| "failed to paint surface to context")?;

                logs.push(format!(
                    "double blur written to surface {:?}",
                    time.elapsed()
                ));
                self.obj().queue_draw();
            } else if let Some(widget) = &*widget_to_render.borrow() {
                self.obj().propagate_draw(widget, cr);
                logs.push(format!("static widget drawn {:?}", time.elapsed()));
            }

            //reset
            cr.reset_clip();
            gtk::render_frame(&self.obj().style_context(), cr, 0.0, 0.0, self_w, self_h);

            cr.restore()?;
        };

        if let Err(err) = res {
            error!("{err}");
        }

        logs.push(format!("total: {:?}", start.elapsed()));

        if start.elapsed() > Duration::from_millis(16) {
            let mut out = String::from("\n");
            for log in logs {
                out.push_str(&log);
                out.push('\n');
            }
            debug!("{out}"); //TODO maybe create a utility library
        }
        glib::Propagation::Proceed
    }
}

pub fn begin_draw_scaled_clip(
    cr: &gdk::cairo::Context,
    (self_w, self_h): (f64, f64),
    (inner_w, inner_h): (f64, f64),
    (scale_x, scale_y): (f64, f64),
    radius: f64,
) {
    cr.arc(
        (self_w - inner_w * scale_x) / 2.0 + radius,
        (self_h - inner_h * scale_y) / 2.0 + radius,
        radius,
        PI * 1.0,
        PI * 1.5,
    );
    //top left //WHY are the angles rotated by 90 degrees
    cr.line_to(
        self_w - (self_w - inner_w * scale_x) / 2.0 - radius,
        (self_h - inner_h * scale_y) / 2.0,
    );
    cr.arc(
        self_w - (self_w - inner_w * scale_x) / 2.0 - radius,
        (self_h - inner_h * scale_y) / 2.0 + radius,
        radius,
        PI * 1.5,
        PI * 0.0,
    );
    //top right
    cr.line_to(
        self_w - (self_w - inner_w * scale_x) / 2.0,
        self_h - (self_h - inner_h * scale_y) / 2.0 - radius,
    );
    cr.arc(
        self_w - (self_w - inner_w * scale_x) / 2.0 - radius,
        self_h - (self_h - inner_h * scale_y) / 2.0 - radius,
        radius,
        PI * 0.0,
        PI * 0.5,
    );
    //bottom right
    cr.line_to(
        (self_w - inner_w * scale_x) / 2.0 + radius,
        self_h - (self_h - inner_h * scale_y) / 2.0,
    );
    cr.arc(
        (self_w - inner_w * scale_x) / 2.0 + radius,
        self_h - (self_h - inner_h * scale_y) / 2.0 - radius,
        radius,
        PI * 0.5,
        PI * 1.0,
    );
    //bottom left
    cr.line_to(
        (self_w - inner_w * scale_x) / 2.0,
        (self_h - inner_h * scale_y) / 2.0 + radius,
    );
    cr.clip();
}

pub fn begin_draw_clip(
    cr: &gdk::cairo::Context,
    (self_w, self_h): (f64, f64),
    (inner_w, inner_h): (f64, f64),
    radius: f64,
) {
    cr.arc(
        (self_w - inner_w) / 2.0 + radius,
        (self_h - inner_h) / 2.0 + radius,
        radius,
        PI * 1.0,
        PI * 1.5,
    );
    //top left //WHY are the angles rotated by 90 degrees
    cr.line_to(
        self_w - (self_w - inner_w) / 2.0 - radius,
        (self_h - inner_h) / 2.0,
    );
    cr.arc(
        self_w - (self_w - inner_w) / 2.0 - radius,
        (self_h - inner_h) / 2.0 + radius,
        radius,
        PI * 1.5,
        PI * 0.0,
    );
    //top right
    cr.line_to(
        self_w - (self_w - inner_w) / 2.0,
        self_h - (self_h - inner_h) / 2.0 - radius,
    );
    cr.arc(
        self_w - (self_w - inner_w) / 2.0 - radius,
        self_h - (self_h - inner_h) / 2.0 - radius,
        radius,
        PI * 0.0,
        PI * 0.5,
    );
    //bottom right
    cr.line_to(
        (self_w - inner_w) / 2.0 + radius,
        self_h - (self_h - inner_h) / 2.0,
    );
    cr.arc(
        (self_w - inner_w) / 2.0 + radius,
        self_h - (self_h - inner_h) / 2.0 - radius,
        radius,
        PI * 0.5,
        PI * 1.0,
    );
    //bottom left
    cr.line_to((self_w - inner_w) / 2.0, (self_h - inner_h) / 2.0 + radius);
    cr.clip();
}
