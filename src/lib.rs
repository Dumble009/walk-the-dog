use rand::thread_rng;
use rand::Rng;
use std::rc::Rc;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

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

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document
        .get_element_by_id("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();

    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    wasm_bindgen_futures::spawn_local(async move {
        let image = web_sys::HtmlImageElement::new().unwrap();
        let (success_tx, success_rx) = futures::channel::oneshot::channel::<Result<(), JsValue>>();
        let success_tx = Rc::new(Mutex::new(Some(success_tx)));
        let error_tx = Rc::clone(&success_tx);
        let callback = Closure::once(move || {
            if let Some(success_tx) = success_tx.lock().ok().and_then(|mut opt| opt.take()) {
                success_tx.send(Ok(()));
            }
            web_sys::console::log_1(&JsValue::from_str("loaded!"));
        });
        let error_callback = Closure::once(move |err| {
            if let Some(error_tx) = error_tx.lock().ok().and_then(|mut opt| opt.take()) {
                error_tx.send(Err(err));
            }
        });
        image.set_onload(Some(callback.as_ref().unchecked_ref()));
        image.set_onerror(Some(error_callback.as_ref().unchecked_ref()));
        image.set_src("Idle (1).png");
        success_rx.await;
        context.draw_image_with_html_image_element(&image, 0.0, 0.0);

        draw_sierpinski(
            &context,
            [(300.0, 0.0), (0.0, 600.0), (600.0, 600.0)],
            (0, 255, 0),
            5,
        );
    });
    web_sys::console::log_1(&JsValue::from_str("waiting..."));
    Ok(())
}
