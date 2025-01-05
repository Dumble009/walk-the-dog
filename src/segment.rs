use crate::engine::{Cell, Image, Point, Rect, Renderer, SpriteSheet};
use std::rc::Rc;
use web_sys::HtmlImageElement;

// 障害物とインタラクトするオブジェクトが実装するトレイト
pub trait Disturbee {
    fn bounding_box(&self) -> Rect;
    fn velocity_y(&self) -> i16;
    fn pos_y(&self) -> i16;
    fn land_on(&mut self, pos: i16);
    fn knock_out(&mut self);
}

pub trait Obstacle {
    fn check_intersection(&self, disturbee: &mut dyn Disturbee);
    fn draw(&self, renderer: &Renderer);
    fn move_horizontally(&mut self, x: i16);
    fn right(&self) -> i16;
}

struct Platform {
    sheet: Rc<SpriteSheet>,
    position: Point,
    bounding_boxes: Vec<Rect>,
    sprites: Vec<Cell>,
}

impl Platform {
    fn new(
        sheet: Rc<SpriteSheet>,
        position: Point,
        sprite_names: &[&str],
        bounding_boxes: &[Rect],
    ) -> Self {
        let sprites = sprite_names
            .iter()
            .filter_map(|sprite_name| sheet.cell(sprite_name).cloned())
            .collect();

        let bounding_boxes = bounding_boxes
            .iter()
            .map(|bounding_box| {
                Rect::new_from_x_y(
                    bounding_box.x() + position.x,
                    bounding_box.y() + position.y,
                    bounding_box.width,
                    bounding_box.height,
                )
            })
            .collect();

        Platform {
            sheet: sheet,
            position: position,
            sprites: sprites,
            bounding_boxes: bounding_boxes,
        }
    }

    fn bounding_boxes(&self) -> &Vec<Rect> {
        &self.bounding_boxes
    }

    fn intersects(&self, rect: &Rect) -> Option<Rect> {
        for bb in &self.bounding_boxes {
            if bb.intersects(rect) {
                return Some(*bb);
            }
        }

        return None;
    }
}

impl Obstacle for Platform {
    fn draw(&self, renderer: &Renderer) {
        let mut x = 0;
        self.sprites.iter().for_each(|sprite| {
            self.sheet.draw(
                renderer,
                &Rect::new_from_x_y(
                    sprite.frame.x,
                    sprite.frame.y,
                    sprite.frame.w,
                    sprite.frame.h,
                ),
                &Rect::new_from_x_y(
                    self.position.x + x,
                    self.position.y,
                    sprite.frame.w,
                    sprite.frame.h,
                ),
            );
            x += sprite.frame.w;
        });

        self.bounding_boxes.iter().for_each(|bb| {
            renderer.draw_bounding_box(bb);
        });
    }

    fn move_horizontally(&mut self, x: i16) {
        self.position.x += x;
        self.bounding_boxes.iter_mut().for_each(|bounding_box| {
            bounding_box.set_x(bounding_box.position.x + x);
        });
    }

    fn check_intersection(&self, disturbee: &mut dyn Disturbee) {
        if let Some(box_to_land_on) = self.intersects(&disturbee.bounding_box()) {
            if disturbee.velocity_y() > 0 && disturbee.pos_y() < self.position.y {
                disturbee.land_on(box_to_land_on.y());
            } else {
                disturbee.knock_out();
            }
        }
    }

    fn right(&self) -> i16 {
        self.bounding_boxes()
            .last()
            .unwrap_or(&Rect::default())
            .right()
    }
}

pub struct Barrier {
    image: Image,
}

impl Barrier {
    pub fn new(image: Image) -> Self {
        Barrier { image }
    }
}

impl Obstacle for Barrier {
    fn check_intersection(&self, disturbee: &mut dyn Disturbee) {
        if disturbee
            .bounding_box()
            .intersects(self.image.bounding_box())
        {
            disturbee.knock_out();
        }
    }

    fn draw(&self, renderer: &Renderer) {
        self.image.draw(renderer);
    }

    fn move_horizontally(&mut self, x: i16) {
        self.image.move_horizontally(x)
    }

    fn right(&self) -> i16 {
        self.image.right()
    }
}

const STONE_ON_GROUND: i16 = 546;
const LOW_PLATFORM: i16 = 420;
const HIGH_PLATFORM: i16 = 375;

pub fn stone_and_platform(
    stone: HtmlImageElement,
    sprite_sheet: Rc<SpriteSheet>,
    offset_x: i16,
) -> Vec<Box<dyn Obstacle>> {
    const INITIAL_STONE_OFFSET: i16 = 150;
    const FIRST_PLATFORM: i16 = 370;
    vec![
        Box::new(Barrier::new(Image::new(
            stone,
            Point {
                x: offset_x + INITIAL_STONE_OFFSET,
                y: STONE_ON_GROUND,
            },
        ))),
        Box::new(create_floating_platform(
            sprite_sheet,
            Point {
                x: offset_x + FIRST_PLATFORM,
                y: LOW_PLATFORM,
            },
        )),
    ]
}

pub fn platform_and_stone(
    stone: HtmlImageElement,
    sprite_sheet: Rc<SpriteSheet>,
    offset_x: i16,
) -> Vec<Box<dyn Obstacle>> {
    const INITIAL_PLATFORM_OFFSET: i16 = 150;
    const FIRST_STONE: i16 = 370;
    vec![
        Box::new(create_floating_platform(
            sprite_sheet,
            Point {
                x: offset_x + INITIAL_PLATFORM_OFFSET,
                y: HIGH_PLATFORM,
            },
        )),
        Box::new(Barrier::new(Image::new(
            stone,
            Point {
                x: offset_x + FIRST_STONE,
                y: STONE_ON_GROUND,
            },
        ))),
    ]
}

fn create_floating_platform(sprite_sheet: Rc<SpriteSheet>, position: Point) -> Platform {
    const FLOATING_PLATFORM_SPRITES: &[&str] = &["13.png", "14.png", "15.png"];
    const FLOATING_PLATFORM_BOUNDING_BOXES: &[Rect] = &[
        Rect::new_from_x_y(0, 0, 60, 54),
        Rect::new_from_x_y(60, 0, 384 - (60 * 2), 93),
        Rect::new_from_x_y(384 - 60, 0, 60, 54),
    ];
    Platform::new(
        sprite_sheet,
        position,
        &FLOATING_PLATFORM_SPRITES,
        &FLOATING_PLATFORM_BOUNDING_BOXES,
    )
}
