use std::time::Duration;

use bevy::DefaultPlugins;
use bevy::color::palettes::tailwind::GRAY_500;
use bevy::prelude::*;
use bevy::window::WindowMode;
use shieldtank::prelude::*;
use tinyrand::{RandRange as _, StdRand};

const AXE_MAN_IID: Iid = iid!("a0170640-9b00-11ef-aa23-11f9c6be2b6e");
const AXE_MAN_IID_U128: u128 = AXE_MAN_IID.as_u128();
const LANCER_IID: Iid = iid!("85f22ca0-fec0-11ee-8cdd-41f7def1ae8a");
const LANCER_IID_U128: u128 = LANCER_IID.as_u128();

const WINDOW_RESOLUTION: UVec2 = UVec2::new(1280, 960);

const GLOBAL_FRAME_TIME: f32 = 1.0 / 3.75;
const ATTACK_FRAME_TIME: f32 = 1.0 / 15.0;
const DYING_FRAME_TIME: f32 = 1.0 / 3.75;

const PLAYER_MOVE_SPEED: f32 = 40.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, States)]
enum GameState {
    #[default]
    Title,
    Playing,
    GameOver,
}

#[derive(Resource)]
struct GlobalTimer {
    timer: Timer,
    frame: usize,
}

impl GlobalTimer {
    fn new() -> Self {
        Self {
            timer: Timer::new(
                Duration::from_secs_f32(GLOBAL_FRAME_TIME),
                TimerMode::Repeating,
            ),
            frame: 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Component)]
enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    pub fn as_vec2(&self) -> Vec2 {
        match self {
            Direction::North => Vec2::new(0.0, 1.0),
            Direction::East => Vec2::new(1.0, 0.0),
            Direction::South => Vec2::new(0.0, -1.0),
            Direction::West => Vec2::new(-1.0, 0.0),
        }
    }
}

#[derive(Clone, PartialEq, Component)]
enum Animation {
    Idle,
    Walking,
    Attack { timer: Timer, frame: usize },
    Dying { timer: Timer, frame: usize },
    Dead,
}

impl Animation {
    fn new_attack() -> Self {
        Animation::Attack {
            timer: Timer::new(
                Duration::from_secs_f32(ATTACK_FRAME_TIME),
                TimerMode::Repeating,
            ),
            frame: 0,
        }
    }

    fn new_dying() -> Self {
        Animation::Dying {
            timer: Timer::new(
                Duration::from_secs_f32(DYING_FRAME_TIME),
                TimerMode::Repeating,
            ),
            frame: 0,
        }
    }

    fn next_animation(&self, global_timer: &GlobalTimer, time: &Time) -> (Self, usize) {
        match self {
            Animation::Idle => (Animation::Idle, global_timer.frame),
            Animation::Walking => (Animation::Walking, global_timer.frame),
            Animation::Dead => (Animation::Dead, 3),
            Animation::Attack { timer, frame } => {
                let mut timer = timer.clone();
                let mut frame = *frame;

                timer.tick(time.delta());

                if timer.just_finished() {
                    frame += 1;
                }

                if frame == 4 {
                    (Animation::Idle, global_timer.frame)
                } else {
                    (Animation::Attack { timer, frame }, frame)
                }
            }
            Animation::Dying { timer, frame } => {
                let mut timer = timer.clone();
                let mut frame = *frame;

                timer.tick(time.delta());

                if timer.just_finished() {
                    frame += 1;
                }

                if frame == 4 {
                    (Animation::Dead, 3)
                } else {
                    (Animation::Dying { timer, frame }, frame)
                }
            }
        }
    }
}

#[derive(Component, Deref, DerefMut)]
struct PlayerMove(Vec2);

#[derive(Component)]
struct MessageBoard;

fn global_timer(time: Res<Time>, mut global_timer: ResMut<GlobalTimer>) {
    global_timer.timer.tick(time.delta());

    if global_timer.timer.just_finished() {
        global_timer.frame += 1;
        global_timer.frame %= 4;
    }
}

fn startup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Transform::from_xyz(0.0, -128.0, 0.0).with_scale(Vec2::splat(0.4).extend(1.0)),
    ));
}

fn init_title_state(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(ShieldtankWorld {
        handle: asset_server.load("ldtk/axe_man_adventure.ldtk#world:Title"),
    });

    commands.spawn((
        Name::new("MessageBoard"),
        Text::new("Press F or Space to start!"),
        TextFont {
            font: FontSource::Handle(asset_server.load("fonts/Primitive.ttf")),
            font_size: 50.0,
            ..Default::default()
        },
        TextColor(GRAY_500.into()),
        TextLayout::new_with_justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(40.0),
            left: Val::Px(5.0),
            right: Val::Px(5.0),
            ..default()
        },
        MessageBoard,
    ));
}

fn key_events_title_state(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.any_just_pressed([KeyCode::KeyF, KeyCode::Space]) {
        next_state.set(GameState::Playing);
    }
}

fn exit_title_state(title_screen: Single<Entity, With<ShieldtankWorld>>, mut commands: Commands) {
    commands.entity(*title_screen).despawn();
}

fn init_playing_state(
    asset_server: Res<AssetServer>,
    mut message_board: Single<&mut Text, With<MessageBoard>>,
    mut commands: Commands,
) {
    commands.spawn(ShieldtankWorld {
        handle: asset_server.load("ldtk/axe_man_adventure.ldtk#world:World"),
    });

    message_board.0 = "The Axe Man begins his adventure!".to_string();
}

fn movement_key_events(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    grid_value_query: GridValueQuery,
    other_entities_query: QueryByGlobalBounds<(Entity, &ShieldtankIid), With<ShieldtankEntity>>,
    axe_man_query: SingleByIid<
        AXE_MAN_IID_U128,
        (Entity, ShieldtankLocation),
        (With<ShieldtankEntity>, Without<PlayerMove>),
    >,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mut message_board: Single<&mut Text, With<MessageBoard>>,
) {
    const UP: (bool, bool, bool, bool) = (true, false, false, false);
    const RIGHT: (bool, bool, bool, bool) = (false, true, false, false);
    const DOWN: (bool, bool, bool, bool) = (false, false, true, false);
    const LEFT: (bool, bool, bool, bool) = (false, false, false, true);

    let up_key = keyboard_input.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
    let right_key = keyboard_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);
    let down_key = keyboard_input.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
    let left_key = keyboard_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);

    let direction = (up_key, right_key, down_key, left_key);

    let direction = match direction {
        UP => Direction::North,
        RIGHT => Direction::East,
        DOWN => Direction::South,
        LEFT => Direction::West,
        _ => return,
    };

    let (axe_man, ref location) = *axe_man_query;

    commands.entity(axe_man).insert(direction);

    let destination = location.get() + 16.0 * direction.as_vec2();

    if let Ok((touched_entity, &iid)) = other_entities_query.single_by_location(destination) {
        // Touching the lancer means death!
        if iid == LANCER_IID {
            info!("The Axe Man runs into The Lancer!");
            message_board.0 = "The axe man has been slain!".to_string();
            commands.entity(axe_man).insert(Animation::new_dying());
            commands
                .entity(touched_entity)
                .insert(Animation::new_attack());
            next_state.set(GameState::GameOver);
        }

        // If it's any other entity besides the lancer, then disallow movement.
        return;
    }

    let Some(terrain_identifier) = grid_value_query.identifier_at(destination) else {
        info!("The Axe Man is wandering in the void...");
        return;
    };

    match terrain_identifier {
        // Cannot walk on water.
        "water" => {}
        // Terrain types which can be walked on
        terrain_identifier
            if terrain_identifier == "grass"
                || terrain_identifier == "dirt"
                || terrain_identifier == "tree"
                || terrain_identifier == "bridge" =>
        {
            let player_move = PlayerMove(destination);
            commands
                .entity(axe_man)
                .insert(player_move)
                .insert(Animation::Walking);
        }
        // Terrain types which we don't have a case for. We emit a warning and don't allow
        // movement.
        unknown => warn!("The Axe Man has encountered some unknown {unknown} terrain?"),
    }
}

fn interact_key_events(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    other_entities_query: QueryByGlobalBounds<
        (Entity, Option<&ShieldtankFieldInstances>, &ShieldtankIid),
        (With<ShieldtankEntity>, Without<PlayerMove>),
    >,
    axe_man_query: SingleByIid<
        AXE_MAN_IID_U128,
        (Entity, ShieldtankLocation, &Direction),
        (With<ShieldtankEntity>, Without<PlayerMove>),
    >,
    mut rand: Local<StdRand>,
    mut next_state: ResMut<NextState<GameState>>,
    mut message_board: Single<&mut Text, With<MessageBoard>>,
    mut commands: Commands,
) {
    if keyboard_input.any_just_pressed([KeyCode::KeyF, KeyCode::Space]) {
        let (axe_man, ref location, direction) = *axe_man_query;

        let location = location.get();
        let target = location + direction.as_vec2() * 16.0;

        if let Ok((touched_entity, field_instances, &iid)) =
            other_entities_query.single_by_location(target)
        {
            if iid == LANCER_IID {
                message_board.0 = "The Vile Lancer has been vanquished!".to_string();
                next_state.set(GameState::GameOver);
                commands.entity(axe_man).insert(Animation::new_attack());
                commands
                    .entity(touched_entity)
                    .insert(Animation::new_dying());
            } else if let Some(field_instances) = field_instances {
                let Some(replies) = field_instances.get_array_string("Replies") else {
                    return;
                };

                let rand = rand.next_range(0..replies.len());
                message_board.0 = replies[rand].clone();
            }
        } else {
            commands.entity(axe_man).insert(Animation::new_attack());
        }
    }
}

fn insert_game_entity_components(
    query: Query<(Entity, &Name), (With<ShieldtankEntity>, Without<Direction>)>,
    mut commands: Commands,
) {
    for (entity, name) in query {
        info!("Setting up the adventure for {name}");
        commands
            .entity(entity)
            .insert(Direction::East)
            .insert(Animation::Idle);
    }
}

fn animate_water(
    global_timer: Res<GlobalTimer>,
    mut query: Query<(&Name, &mut Visibility), With<ShieldtankLayer>>,
) {
    let frame = global_timer.frame;

    for (name, mut visibility) in query.iter_mut() {
        let target_frame = match name.as_str() {
            "Water1" => 0,
            "Water2" => 1,
            "Water3" => 2,
            "Water4" => 3,
            _ => continue,
        };

        if target_frame == frame {
            *visibility = Visibility::Inherited;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn animate_entities(
    time: Res<Time>,
    global_timer: Res<GlobalTimer>,
    mut query: Query<(
        &ShieldtankFieldInstances,
        &Direction,
        &mut Animation,
        &mut ShieldtankTile,
    )>,
) {
    for (field_instances, direction, mut animation, mut tile) in query.iter_mut() {
        let (next_animation, frame) = animation.next_animation(&global_timer, &time);

        let prefix = match next_animation {
            Animation::Idle => "Idle",
            Animation::Walking => "Walk",
            Animation::Attack { .. } => "Attack",
            Animation::Dying { .. } | Animation::Dead => "Dead",
        };

        let suffix = match direction {
            Direction::North => "North",
            Direction::East => "Profile",
            Direction::South => "South",
            Direction::West => "Profile",
        };

        let identifier = format!("{}{}", prefix, suffix);

        let flip_x = *direction == Direction::West;

        *animation = next_animation;

        let Some(tiles) = field_instances.get_array_tile(&identifier) else {
            error!("missing tiles field instance? {identifier}");
            return;
        };

        let Some(mut new_tile) = tiles.get(frame).cloned() else {
            error!("bad frame! {frame}");
            return;
        };

        new_tile.flip_x(flip_x);

        *tile = new_tile;
    }
}

fn lancer_face_axe_man(
    axe_man_query: SingleByIid<
        AXE_MAN_IID_U128,
        ShieldtankLocation,
        Changed<ShieldtankGlobalBounds>,
    >,
    lancer_query: SingleByIid<LANCER_IID_U128, (ShieldtankLocation, &mut Direction)>,
) {
    let axe_man_location = &*axe_man_query;

    let (lancer_location, mut lancer_direction) = lancer_query.into_inner();

    let dir_vec = axe_man_location.get() - lancer_location.get();

    let direction = match (dir_vec.x < dir_vec.y, -dir_vec.x < dir_vec.y) {
        (true, true) => Direction::North,
        (true, false) => Direction::West,
        (false, true) => Direction::East,
        (false, false) => Direction::South,
    };

    *lancer_direction = direction;
}

fn player_move(
    time: Res<Time>,
    query: Query<(Entity, ShieldtankLocationMut, &PlayerMove), With<ShieldtankEntity>>,
    mut commands: Commands,
) {
    for (entity, mut location, player_move) in query {
        let to_destination = **player_move - location.get();

        if to_destination.length_squared() < 0.1 {
            location.add(to_destination);
            commands
                .entity(entity)
                .remove::<PlayerMove>()
                .insert(Animation::Idle);
        } else {
            let step = to_destination.normalize() * PLAYER_MOVE_SPEED * time.delta_secs();
            location.add(step);
        }
    }
}

fn gameover_key_events(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.any_just_pressed([KeyCode::KeyF, KeyCode::Space]) {
        next_state.set(GameState::Title);
    }
}

fn exit_gameover_state(
    world: Single<Entity, With<ShieldtankWorld>>,
    message_board: Single<Entity, With<MessageBoard>>,
    mut commands: Commands,
) {
    commands.entity(*world).despawn();
    commands.entity(*message_board).despawn();
}

fn main() {
    let log_plugin_settings = bevy::log::LogPlugin {
        level: bevy::log::Level::WARN,
        filter: "wgpu_hal=off,\
                 winit=off,\
                 bevy_winit=off,\
                 bevy_ldtk_asset=info,\
                 shieldtank=debug,\
                 axe_man_adventure=debug"
            .into(),
        ..default()
    };

    let window_plugin_settings: WindowPlugin = WindowPlugin {
        primary_window: Some(Window {
            mode: WindowMode::Windowed,
            resolution: WINDOW_RESOLUTION.into(),
            resizable: false,
            ..Default::default()
        }),
        ..Default::default()
    };

    let image_plugin_settings = ImagePlugin::default_nearest();

    let asset_plugin_settings = AssetPlugin {
        // To make itch.io happy, since they're allergic to 404 errors.
        meta_check: bevy::asset::AssetMetaCheck::Never,
        ..Default::default()
    };

    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins
            .set(log_plugin_settings)
            .set(window_plugin_settings)
            .set(image_plugin_settings)
            .set(asset_plugin_settings),
        ShieldtankPlugins,
    ))
    .init_state::<GameState>()
    .insert_resource(GlobalTimer::new())
    .add_systems(Startup, startup)
    .add_systems(Update, global_timer);

    #[cfg(debug_assertions)]
    {
        // use bevy_inspector_egui::bevy_egui::EguiPlugin;
        // use bevy_inspector_egui::quick::WorldInspectorPlugin;
        // app.add_plugins(EguiPlugin::default())
        //     .add_plugins(WorldInspectorPlugin::default());
    }

    // Title state
    app.add_systems(OnEnter(GameState::Title), init_title_state);
    app.add_systems(
        Update,
        key_events_title_state.run_if(in_state(GameState::Title)),
    );
    app.add_systems(OnExit(GameState::Title), exit_title_state);

    // Playing state
    app.add_systems(OnEnter(GameState::Playing), init_playing_state);
    app.add_systems(
        Update,
        (
            movement_key_events,
            interact_key_events,
            insert_game_entity_components,
            animate_water,
            animate_entities,
            lancer_face_axe_man,
            player_move,
        )
            .run_if(in_state(GameState::Playing)),
    );

    // Game Over state
    app.add_systems(
        Update,
        (gameover_key_events, animate_water, animate_entities)
            .run_if(in_state(GameState::GameOver)),
    );
    app.add_systems(OnExit(GameState::GameOver), exit_gameover_state);

    app.run();
}
