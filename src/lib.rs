use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::console;

// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn draw_sierpinski(
    context: &web_sys::CanvasRenderingContext2d,
    points: [(f64, f64); 3],
    depth: u32,
) {
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
        draw_sierpinski(
            context,
            [points[0], top_left_center, top_right_center],
            depth - 1,
        );
        draw_sierpinski(
            context,
            [top_left_center, points[1], left_right_center],
            depth - 1,
        );
        draw_sierpinski(
            context,
            [top_right_center, left_right_center, points[2]],
            depth - 1,
        );
    } else {
        context.move_to(points[0].0, points[0].1); // top of triangle
        context.begin_path();
        context.line_to(points[1].0, points[1].1); // bottom left of triangle
        context.line_to(points[2].0, points[2].1); // bottom right of triangle
        context.line_to(points[0].0, points[0].1); // back to top of triangle
        context.close_path();
        context.stroke();
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

    draw_sierpinski(&context, [(300.0, 0.0), (0.0, 600.0), (600.0, 600.0)], 5);
    Ok(())
}
