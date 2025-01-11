use self::red_hat_boy_states::*;
use crate::browser;
use crate::engine;
use crate::engine::Audio;
use crate::engine::KeyState;
use crate::engine::Sound;
use crate::engine::SpriteSheet;
use crate::engine::{Cell, Game, Image, Point, Rect, Renderer, Sheet};
use crate::segment::{platform_and_stone, stone_and_platform, Disturbee, Obstacle};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rand::{thread_rng, Rng};
use std::rc::Rc;
use web_sys::HtmlImageElement;

const CANVAS_HEIGHT: i16 = 600;
const TIMELINE_MINIMUM: i16 = 1000;
const OBSTACLE_BUFFER: i16 = 20;

struct Walk {
    obstacle_sheet: Rc<SpriteSheet>,
    boy: RedHatBoy,
    backgrounds: [Image; 2],
    obstacles: Vec<Box<dyn Obstacle>>,
    stone: HtmlImageElement,
    timeline: i16,
}

impl Walk {
    fn velocity(&self) -> i16 {
        -self.boy.walking_speed()
    }

    fn generate_next_segment(&mut self) {
        let mut rng = thread_rng();
        let next_segment = rng.gen_range(0..2);

        let mut next_obstacles = match next_segment {
            0 => stone_and_platform(
                self.stone.clone(),
                self.obstacle_sheet.clone(),
                self.timeline + OBSTACLE_BUFFER,
            ),
            1 => platform_and_stone(
                self.stone.clone(),
                self.obstacle_sheet.clone(),
                self.timeline + OBSTACLE_BUFFER,
            ),
            _ => vec![],
        };

        self.timeline = rightmost(&next_obstacles);
        self.obstacles.append(&mut next_obstacles);
    }
}

pub enum WalkTheDog {
    Loading,
    Loaded(Walk),
}

impl WalkTheDog {
    pub fn new() -> Self {
        WalkTheDog::Loading
    }
}

enum Event {
    Run,
    Slide,
    Update,
    Jump,
    KnockOut,
    Land(i16),
}

pub struct RedHatBoy {
    state_machine: RedHatBoyStateMachine,
    sprite_sheet: Sheet,
    image: HtmlImageElement,
}

impl RedHatBoy {
    fn new(sheet: Sheet, image: HtmlImageElement, audio: Audio, jump_sound: Sound) -> Self {
        RedHatBoy {
            state_machine: RedHatBoyStateMachine::Idle(RedHatBoyState::new(audio, jump_sound)),
            sprite_sheet: sheet,
            image: image,
        }
    }

    fn frame_name(&self) -> String {
        format!(
            "{} ({}).png",
            self.state_machine.frame_name(),
            (self.state_machine.context().frame / 3) + 1
        )
    }

    fn current_sprite(&self) -> Option<&Cell> {
        self.sprite_sheet.frames.get(&self.frame_name())
    }

    fn draw(&self, renderer: &Renderer) {
        let sprite = self.current_sprite().expect("Cell not found");

        renderer.draw_image(
            &self.image,
            &Rect::new_from_x_y(
                sprite.frame.x.into(),
                sprite.frame.y.into(),
                sprite.frame.w.into(),
                sprite.frame.h.into(),
            ),
            &self.destination_box(),
        );

        renderer.draw_bounding_box(&self.bounding_box());
    }

    fn update(&mut self) {
        self.state_machine = self.state_machine.clone().update();
    }

    fn run_right(&mut self) {
        self.state_machine = self.state_machine.clone().transition(Event::Run);
    }

    fn slide(&mut self) {
        self.state_machine = self.state_machine.clone().transition(Event::Slide);
    }

    fn jump(&mut self) {
        self.state_machine = self.state_machine.clone().transition(Event::Jump);
    }

    fn log_context(&self) {
        log!(
            "position.y : {}, velocity.y : {}",
            self.state_machine.context().position.y,
            self.state_machine.context().velocity.y
        );
    }

    fn destination_box(&self) -> Rect {
        let sprite = self.current_sprite().expect("Cell not found!");

        Rect::new_from_x_y(
            (self.state_machine.context().position.x + sprite.sprite_source_size.x as i16).into(),
            (self.state_machine.context().position.y + sprite.sprite_source_size.y as i16).into(),
            sprite.frame.w.into(),
            sprite.frame.h.into(),
        )
    }

    fn walking_speed(&self) -> i16 {
        self.state_machine.context().velocity.x
    }
}

impl Disturbee for RedHatBoy {
    fn bounding_box(&self) -> Rect {
        const X_OFFSET: i16 = 18;
        const Y_OFFSET: i16 = 14;
        const WIDTH_OFFSET: i16 = 28;
        let destination_box = self.destination_box();
        Rect::new_from_x_y(
            destination_box.x() + X_OFFSET,
            destination_box.y() + Y_OFFSET,
            destination_box.width - WIDTH_OFFSET,
            destination_box.height - Y_OFFSET,
        )
    }

    fn velocity_y(&self) -> i16 {
        self.state_machine.context().velocity.y
    }

    fn pos_y(&self) -> i16 {
        self.state_machine.context().position.y
    }

    fn land_on(&mut self, ground_height: i16) {
        self.state_machine = self
            .state_machine
            .clone()
            .transition(Event::Land(ground_height));
    }

    fn knock_out(&mut self) {
        self.state_machine = self.state_machine.clone().transition(Event::KnockOut);
    }
}

#[derive(Clone)]
enum RedHatBoyStateMachine {
    Idle(RedHatBoyState<Idle>),
    Running(RedHatBoyState<Running>),
    Sliding(RedHatBoyState<Sliding>),
    Jumping(RedHatBoyState<Jumping>),
    Falling(RedHatBoyState<Falling>),
    KnockedOut(RedHatBoyState<KnockedOut>),
}

impl RedHatBoyStateMachine {
    fn transition(self, event: Event) -> Self {
        match (self.clone(), event) {
            (RedHatBoyStateMachine::Idle(state), Event::Run) => state.run().into(),
            (RedHatBoyStateMachine::Running(state), Event::Slide) => state.slide().into(),
            (RedHatBoyStateMachine::Running(state), Event::Jump) => state.jump().into(),
            (RedHatBoyStateMachine::Idle(state), Event::Update) => state.update().into(),
            (RedHatBoyStateMachine::Running(state), Event::Update) => state.update().into(),
            (RedHatBoyStateMachine::Sliding(state), Event::Update) => state.update().into(),
            (RedHatBoyStateMachine::Jumping(state), Event::Update) => state.update().into(),
            (RedHatBoyStateMachine::Falling(state), Event::Update) => state.update().into(),
            (RedHatBoyStateMachine::KnockedOut(state), Event::Update) => state.update().into(),
            (RedHatBoyStateMachine::Running(state), Event::KnockOut) => state.knock_out().into(),
            (RedHatBoyStateMachine::Jumping(state), Event::KnockOut) => state.knock_out().into(),
            (RedHatBoyStateMachine::Sliding(state), Event::KnockOut) => state.knock_out().into(),
            (RedHatBoyStateMachine::Jumping(state), Event::Land(y)) => state.land_on(y).into(),
            (RedHatBoyStateMachine::Running(state), Event::Land(y)) => state.land_on(y).into(),
            (RedHatBoyStateMachine::Sliding(state), Event::Land(y)) => state.land_on(y).into(),
            (RedHatBoyStateMachine::KnockedOut(state), Event::Land(y)) => state.land_on(y).into(),
            _ => self,
        }
    }

    pub fn frame_name(&self) -> &str {
        match self {
            RedHatBoyStateMachine::Idle(state) => state.frame_name(),
            RedHatBoyStateMachine::Running(state) => state.frame_name(),
            RedHatBoyStateMachine::Sliding(state) => state.frame_name(),
            RedHatBoyStateMachine::Jumping(state) => state.frame_name(),
            RedHatBoyStateMachine::Falling(state) => state.frame_name(),
            RedHatBoyStateMachine::KnockedOut(state) => state.frame_name(),
        }
    }

    pub fn context(&self) -> &RedHatBoyContext {
        match self {
            RedHatBoyStateMachine::Idle(state) => &state.context(),
            RedHatBoyStateMachine::Running(state) => &state.context(),
            RedHatBoyStateMachine::Sliding(state) => &state.context(),
            RedHatBoyStateMachine::Jumping(state) => &state.context(),
            RedHatBoyStateMachine::Falling(state) => &state.context(),
            RedHatBoyStateMachine::KnockedOut(state) => &state.context(),
        }
    }

    pub fn update(self) -> Self {
        self.transition(Event::Update)
    }
}

impl From<RedHatBoyState<Idle>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Idle>) -> Self {
        RedHatBoyStateMachine::Idle(state)
    }
}

impl From<RedHatBoyState<Running>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Running>) -> Self {
        RedHatBoyStateMachine::Running(state)
    }
}

impl From<RedHatBoyState<Sliding>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Sliding>) -> Self {
        RedHatBoyStateMachine::Sliding(state)
    }
}

impl From<RedHatBoyState<Jumping>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Jumping>) -> Self {
        RedHatBoyStateMachine::Jumping(state)
    }
}

impl From<RedHatBoyState<Falling>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Falling>) -> Self {
        RedHatBoyStateMachine::Falling(state)
    }
}

impl From<SlidingEndState> for RedHatBoyStateMachine {
    fn from(end_state: SlidingEndState) -> Self {
        match end_state {
            SlidingEndState::Complete(running_state) => running_state.into(),
            SlidingEndState::Sliding(sliding_state) => sliding_state.into(),
        }
    }
}

impl From<JumpingEndState> for RedHatBoyStateMachine {
    fn from(end_state: JumpingEndState) -> Self {
        match end_state {
            JumpingEndState::Complete(running_state) => running_state.into(),
            JumpingEndState::Jumping(jumping_state) => jumping_state.into(),
        }
    }
}

impl From<RedHatBoyState<KnockedOut>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<KnockedOut>) -> Self {
        RedHatBoyStateMachine::KnockedOut(state)
    }
}

impl From<FallingEndState> for RedHatBoyStateMachine {
    fn from(end_state: FallingEndState) -> Self {
        match end_state {
            FallingEndState::KnockedOut(knocked_out_state) => knocked_out_state.into(),
            FallingEndState::Falling(falling_state) => falling_state.into(),
        }
    }
}

mod red_hat_boy_states {
    use crate::engine::Audio;
    use crate::engine::Point;
    use crate::engine::Sound;

    use super::RedHatBoyStateMachine;
    const FLOOR: i16 = 479;
    const STARTING_POINT: i16 = -20;
    const IDLE_FRAME_NAME: &str = "Idle";
    const RUN_FRAME_NAME: &str = "Run";
    const SLIDING_FRAME_NAME: &str = "Slide";
    const JUMPING_FRAME_NAME: &str = "Jump";
    const IDLE_FRAMES: u8 = 29;
    const RUNNING_FRAMES: u8 = 23;
    const SLIDING_FRAMES: u8 = 14;
    const JUMPING_FRAMES: u8 = 35;
    const FALLING_FRAMES: u8 = 29;
    const FALLING_FRAME_NAME: &str = "Dead";
    const RUNNING_SPEED: i16 = 4;
    const JUMP_SPEED: i16 = -25;
    const GRAVITY: i16 = 1;
    use super::CANVAS_HEIGHT;
    const PLAYER_HEIGHT: i16 = CANVAS_HEIGHT - FLOOR;
    const FALLING_TERMINAL_SPEED: i16 = 20;

    #[derive(Clone)]
    pub struct RedHatBoyState<S> {
        context: RedHatBoyContext,
        _state: S,
    }

    impl<S> RedHatBoyState<S> {
        pub fn context(&self) -> &RedHatBoyContext {
            &self.context
        }
    }

    #[derive(Clone)]
    pub struct RedHatBoyContext {
        pub frame: u8,
        pub position: Point,
        pub velocity: Point,
        audio: Audio,
        jump_sound: Sound,
    }

    #[derive(Copy, Clone)]
    pub struct Idle;

    #[derive(Copy, Clone)]
    pub struct Running;

    #[derive(Copy, Clone)]
    pub struct Sliding;

    #[derive(Copy, Clone)]
    pub struct Jumping;

    #[derive(Copy, Clone)]
    pub struct Falling;

    #[derive(Copy, Clone)]
    pub struct KnockedOut;

    impl RedHatBoyState<Idle> {
        pub fn new(audio: Audio, jump_sound: Sound) -> Self {
            RedHatBoyState {
                context: RedHatBoyContext {
                    frame: 0,
                    position: Point {
                        x: STARTING_POINT,
                        y: FLOOR,
                    },
                    velocity: Point { x: 0, y: 0 },
                    audio,
                    jump_sound,
                },
                _state: Idle {},
            }
        }

        pub fn run(self) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context.reset_frame().run_right(),
                _state: Running {},
            }
        }

        pub fn frame_name(&self) -> &str {
            IDLE_FRAME_NAME
        }

        pub fn update(mut self) -> Self {
            self.context = self.context.update(IDLE_FRAMES);
            self
        }
    }

    impl RedHatBoyState<Running> {
        pub fn frame_name(&self) -> &str {
            RUN_FRAME_NAME
        }

        pub fn update(mut self) -> Self {
            self.context = self.context.update(RUNNING_FRAMES);
            self
        }

        pub fn slide(self) -> RedHatBoyState<Sliding> {
            RedHatBoyState {
                context: self.context.reset_frame(),
                _state: Sliding {},
            }
        }

        pub fn jump(self) -> RedHatBoyState<Jumping> {
            RedHatBoyState {
                context: self
                    .context
                    .set_vertical_velocity(JUMP_SPEED)
                    .reset_frame()
                    .play_jump_sound(),
                _state: Jumping {},
            }
        }

        pub fn knock_out(self) -> RedHatBoyState<Falling> {
            RedHatBoyState {
                context: self.context.reset_frame().stop(),
                _state: Falling {},
            }
        }

        pub fn land_on(self, position: i16) -> Self {
            RedHatBoyState {
                context: self.context.set_on(position),
                _state: Running,
            }
        }
    }

    pub enum SlidingEndState {
        Complete(RedHatBoyState<Running>),
        Sliding(RedHatBoyState<Sliding>),
    }

    impl RedHatBoyState<Sliding> {
        pub fn frame_name(&self) -> &str {
            SLIDING_FRAME_NAME
        }

        pub fn update(mut self) -> SlidingEndState {
            self.context = self.context.update(SLIDING_FRAMES);

            if self.context.frame >= SLIDING_FRAMES {
                SlidingEndState::Complete(self.stand())
            } else {
                SlidingEndState::Sliding(self)
            }
        }

        fn stand(self) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context.reset_frame(),
                _state: Running,
            }
        }

        pub fn knock_out(self) -> RedHatBoyState<Falling> {
            RedHatBoyState {
                context: self.context.reset_frame().stop(),
                _state: Falling {},
            }
        }

        pub fn land_on(self, position: i16) -> Self {
            RedHatBoyState {
                context: self.context.set_on(position as i16),
                _state: Sliding,
            }
        }
    }

    pub enum JumpingEndState {
        Complete(RedHatBoyState<Running>),
        Jumping(RedHatBoyState<Jumping>),
    }

    impl RedHatBoyState<Jumping> {
        pub fn frame_name(&self) -> &str {
            JUMPING_FRAME_NAME
        }

        pub fn update(mut self) -> JumpingEndState {
            self.context = self.context.update(JUMPING_FRAMES);
            if self.context.position.y >= FLOOR {
                JumpingEndState::Complete(self.land_on(CANVAS_HEIGHT))
            } else {
                JumpingEndState::Jumping(self)
            }
        }

        pub fn land_on(self, position: i16) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context.reset_frame().set_on(position as i16),
                _state: Running,
            }
        }

        pub fn knock_out(self) -> RedHatBoyState<Falling> {
            RedHatBoyState {
                context: self.context.reset_frame().stop(),
                _state: Falling {},
            }
        }
    }

    pub enum FallingEndState {
        KnockedOut(RedHatBoyState<KnockedOut>),
        Falling(RedHatBoyState<Falling>),
    }

    impl RedHatBoyState<Falling> {
        pub fn frame_name(&self) -> &str {
            FALLING_FRAME_NAME
        }

        fn down(self) -> RedHatBoyState<KnockedOut> {
            RedHatBoyState {
                context: self.context,
                _state: KnockedOut,
            }
        }

        pub fn update(mut self) -> FallingEndState {
            self.context = self.context.update(FALLING_FRAMES);
            if self.context.frame >= FALLING_FRAMES {
                FallingEndState::KnockedOut(self.down())
            } else {
                FallingEndState::Falling(self)
            }
        }
    }

    impl RedHatBoyState<KnockedOut> {
        pub fn frame_name(&self) -> &str {
            FALLING_FRAME_NAME
        }

        pub fn update(mut self) -> Self {
            self.context = self
                .context
                .update(FALLING_FRAMES)
                .fix_frame(FALLING_FRAMES - 1);

            self
        }

        pub fn land_on(mut self, position: i16) -> Self {
            self.context = self.context.set_on(position);
            self
        }
    }

    impl RedHatBoyContext {
        fn update(mut self, frame_count: u8) -> Self {
            self.velocity.y += GRAVITY;
            if self.velocity.y >= FALLING_TERMINAL_SPEED {
                self.velocity.y = FALLING_TERMINAL_SPEED;
            }

            if self.frame < frame_count {
                self.frame += 1;
            } else {
                self.frame = 0;
            }
            self.position.y += self.velocity.y;
            if self.position.y > FLOOR {
                self.position.y = FLOOR;
            }
            self
        }

        fn reset_frame(mut self) -> Self {
            self.frame = 0;
            self
        }

        fn run_right(mut self) -> Self {
            self.velocity.x += RUNNING_SPEED;
            self
        }

        fn set_vertical_velocity(mut self, y: i16) -> Self {
            log!("set_vertical_velocity");
            self.velocity.y = y;
            self
        }

        fn stop(mut self) -> Self {
            self.velocity.x = 0;
            self
        }

        fn set_on(mut self, position: i16) -> Self {
            log!("set_on");
            let position = position - PLAYER_HEIGHT;
            self.position.y = position;
            self.velocity.y = 0;
            self
        }

        fn fix_frame(mut self, frame: u8) -> Self {
            self.frame = frame;
            self
        }

        fn play_jump_sound(self) -> Self {
            if let Err(err) = self.audio.play_sound(&self.jump_sound) {
                log!("Error playing jump sound {:#?}", err);
            }
            self
        }
    }
}

const LOW_PLATFORM: i16 = 420;
const HIGH_PLATFORM: i16 = 375;

const FIRST_PLATFORM: i16 = 370;

#[async_trait(?Send)]
impl Game for WalkTheDog {
    async fn initialize(&self) -> Result<Box<dyn Game>> {
        match self {
            WalkTheDog::Loading => {
                let json = browser::fetch_json("rhb.json").await?;
                let sheet: Option<Sheet> = serde_wasm_bindgen::from_value(json)
                    .expect("Could not convert rhb.json into a Sheet structure.");
                let image = Some(engine::load_image("rhb.png").await?);
                let background = engine::load_image("BG.png").await?;
                let stone = engine::load_image("Stone.png").await?;

                let audio = Audio::new()?;
                let sound = audio.load_sound("SFX_Jump_23.mp3").await?;

                let rhb = RedHatBoy::new(
                    sheet.clone().ok_or_else(|| anyhow!("No Sheet Present"))?,
                    image.clone().ok_or_else(|| anyhow!("No Imgage Present"))?,
                    audio,
                    sound,
                );

                let json = browser::fetch_json("tiles.json").await?;
                let sheet: Option<Sheet> = serde_wasm_bindgen::from_value(json)
                    .expect("Could not convert tiles.json into a Sheet structure.");

                let sprite_sheet = Rc::new(SpriteSheet::new(
                    sheet.expect("Could not load tiles.json"),
                    engine::load_image("tiles.png").await?,
                ));

                let starting_obstacles = stone_and_platform(stone.clone(), sprite_sheet.clone(), 0);
                let timeline = rightmost(&starting_obstacles);

                let background_width = background.width() as i16;
                Ok(Box::new(WalkTheDog::Loaded(Walk {
                    boy: rhb,
                    backgrounds: [
                        Image::new(background.clone(), Point { x: 0, y: 0 }),
                        Image::new(
                            background,
                            Point {
                                x: background_width,
                                y: 0,
                            },
                        ),
                    ],
                    obstacles: starting_obstacles,
                    obstacle_sheet: sprite_sheet,
                    stone: stone,
                    timeline: timeline,
                })))
            }
            WalkTheDog::Loaded(_) => Err(anyhow!("Error: Game is already initialized!")),
        }
    }

    fn update(&mut self, keystate: &KeyState) {
        if let WalkTheDog::Loaded(walk) = self {
            let mut velocity = Point { x: 0, y: 0 };
            if keystate.is_pressed("ArrowDown") {
                velocity.y += 3;
                walk.boy.slide();
            }

            if keystate.is_pressed("ArrowUp") {
                velocity.y -= 3;
            }

            if keystate.is_pressed("ArrowRight") {
                velocity.x += 3;
                walk.boy.run_right();
            }

            if keystate.is_pressed("ArrowLeft") {
                velocity.x -= 3;
            }

            if keystate.is_pressed("Space") {
                walk.boy.jump();
            }

            walk.boy.update();

            let velocity = walk.velocity();

            let [first_background, second_background] = &mut walk.backgrounds;
            first_background.move_horizontally(velocity);
            second_background.move_horizontally(velocity);

            if first_background.right() < 0 {
                first_background.set_x(second_background.right());
            }

            if second_background.right() < 0 {
                second_background.set_x(first_background.right());
            }

            walk.obstacles.retain(|obstacle| obstacle.right() > 0);

            let boy_ref = &mut walk.boy;
            walk.obstacles.iter_mut().for_each(|obstacle| {
                obstacle.move_horizontally(velocity);
                obstacle.check_intersection(boy_ref);
            });

            if walk.timeline < TIMELINE_MINIMUM {
                walk.generate_next_segment();
            } else {
                walk.timeline += velocity;
            }
        }
    }

    fn draw(&self, renderer: &Renderer) {
        if let WalkTheDog::Loaded(walk) = self {
            renderer.clear(&Rect::new_from_x_y(0, 0, 600, CANVAS_HEIGHT));

            walk.backgrounds.iter().for_each(|background| {
                background.draw(renderer);
            });
            walk.boy.draw(renderer);
            walk.obstacles.iter().for_each(|obstacle| {
                obstacle.draw(renderer);
            });
        }
    }
} // impl Game for WalkTheDog

fn rightmost(obstacle_list: &Vec<Box<dyn Obstacle>>) -> i16 {
    obstacle_list
        .iter()
        .map(|obstacle| obstacle.right())
        .max_by(|x, y| x.cmp(&y))
        .unwrap_or(0)
}
