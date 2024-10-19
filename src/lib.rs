use rand::thread_rng;
use rand::Rng;
use serde::Deserialize;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[macro_use]
mod browser;
mod engine;

// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[derive(Deserialize)]
struct Sheet {
    frames: HashMap<String, Cell>,
}

#[derive(Deserialize)]
struct Rect {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}

#[derive(Deserialize)]
struct Cell {
    frame: Rect,
}

fn draw_triangle(
    context: &web_sys::CanvasRenderingContext2d,
    points: [(f64, f64); 3],
    color: (u8, u8, u8),
) {
    context.move_to(points[0].0, points[0].1); // top of triangle
    context.begin_path();
    context.line_to(points[1].0, points[1].1); // bottom left of triangle
    context.line_to(points[2].0, points[2].1); // bottom right of triangle
    context.line_to(points[0].0, points[0].1); // back to top of triangle
    context.close_path();
    context.stroke();
    let color_str = format!("rgb({}, {}, {})", color.0, color.1, color.2);
    context.set_fill_style_str(&color_str);
    context.fill();
}

fn draw_sierpinski(
    context: &web_sys::CanvasRenderingContext2d,
    points: [(f64, f64); 3],
    color: (u8, u8, u8),
    depth: u32,
) {
    draw_triangle(context, points, color);
    if depth > 0 {
        let top_left_center = (
            (points[0].0 + points[1].0) / 2.0,
            (points[0].1 + points[1].1) / 2.0,
        );
        let top_right_center = (
            (points[0].0 + points[2].0) / 2.0,
            (points[0].1 + points[2].1) / 2.0,
        );
        let left_right_center = (
            (points[1].0 + points[2].0) / 2.0,
            (points[1].1 + points[2].1) / 2.0,
        );

        let mut rng = thread_rng();
        let next_color = (
            rng.gen_range(0..255),
            rng.gen_range(0..255),
            rng.gen_range(0..255),
        );
        draw_sierpinski(
            context,
            [points[0], top_left_center, top_right_center],
            next_color,
            depth - 1,
        );
        draw_sierpinski(
            context,
            [top_left_center, points[1], left_right_center],
            next_color,
            depth - 1,
        );
        draw_sierpinski(
            context,
            [top_right_center, left_right_center, points[2]],
            next_color,
            depth - 1,
        );
    } else {
        draw_triangle(context, points, color);
    }
}

// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let context = browser::context().expect("Could not get browser context");

    browser::spawn_local(async move {
        let json = browser::fetch_json("rhb.json")
            .await
            .expect("Could not featch rhb.json");
        let sheet: Sheet = serde_wasm_bindgen::from_value(json)
            .expect("Could not convert rhb.json into a Sheet structure.");

        let image = engine::load_image("rhb.png")
            .await
            .expect("Could not load rhb.png");

        let mut frame = -1;
        let interval_callback = Closure::wrap(Box::new(move || {
            frame = (frame + 1) % 8;
            let frame_name = format!("Run ({}).png", frame + 1);
            let sprite = sheet.frames.get(&frame_name).expect("Cell not found");
            context.clear_rect(0.0, 0.0, 600.0, 600.0);
            let _ = context
                .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    &image,
                    sprite.frame.x.into(),
                    sprite.frame.y.into(),
                    sprite.frame.w.into(),
                    sprite.frame.h.into(),
                    300.0,
                    300.0,
                    sprite.frame.w.into(),
                    sprite.frame.h.into(),
                );
        }) as Box<dyn FnMut()>);
        let _ = browser::window()
            .unwrap()
            .set_interval_with_callback_and_timeout_and_arguments_0(
                interval_callback.as_ref().unchecked_ref(),
                50,
            );
        interval_callback.forget();
    });
    web_sys::console::log_1(&JsValue::from_str("waiting..."));
    Ok(())
}
