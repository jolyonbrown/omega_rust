use bevy::prelude::*;

const WINDOW_WIDTH: f32 = 800.0;
const WINDOW_HEIGHT: f32 = 600.0;
const BORDER_COLOR: Color = Color::rgb(1.0, 1.0, 0.0); // Yellow
const TEXT_COLOR: Color = Color::rgb(1.0, 1.0, 0.0); // Yellow

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                title: "Omega Rust".to_string(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (player_movement, update_score))
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Resource)]
struct GameState {
    score: u32,
    high_score: u32,
    lives: u8,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera
    commands.spawn(Camera2dBundle::default());

    // Outer border
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: BORDER_COLOR,
            custom_size: Some(Vec2::new(WINDOW_WIDTH, WINDOW_HEIGHT)),
            ..default()
        },
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });

    // Inner border (score area)
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: BORDER_COLOR,
            custom_size: Some(Vec2::new(WINDOW_WIDTH * 0.8, WINDOW_HEIGHT * 0.2)),
            ..default()
        },
        transform: Transform::from_xyz(0.0, WINDOW_HEIGHT * 0.3, 1.0),
        ..default()
    });

    // Score text
    commands.spawn(TextBundle::from_sections([
        TextSection::new(
            "SCORE\n0",
            TextStyle {
                font: asset_server.load("fonts/FiraCode-Medium.ttf"),
                font_size: 20.0,
                color: TEXT_COLOR,
            },
        ),
    ])
    .with_style(Style {
        position_type: PositionType::Absolute,
        top: Val::Px(WINDOW_HEIGHT * 0.4),
        left: Val::Px(WINDOW_WIDTH * 0.1),
        ..default()
    }));

    // High Score text
    commands.spawn(TextBundle::from_sections([
        TextSection::new(
            "HIGH SCORE\n0",
            TextStyle {
                font: asset_server.load("fonts/FiraCode-Medium.ttf"),
                font_size: 20.0,
                color: TEXT_COLOR,
            },
        ),
    ])
    .with_style(Style {
        position_type: PositionType::Absolute,
        top: Val::Px(WINDOW_HEIGHT * 0.4),
        right: Val::Px(WINDOW_WIDTH * 0.1),
        ..default()
    }));

    // Player ship
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: BORDER_COLOR,
                custom_size: Some(Vec2::new(20.0, 20.0)),
                ..default()
            },
            transform: Transform::from_xyz(WINDOW_WIDTH * 0.4, WINDOW_HEIGHT * 0.2, 2.0),
            ..default()
        },
        Player,
    ));

    // Initialize game state
    commands.insert_resource(GameState {
        score: 0,
        high_score: 0,
        lives: 3,
    });
}

fn player_movement(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
    time: Res<Time>,
) {
    let mut player_transform = query.single_mut();
    let mut direction = Vec3::ZERO;

    if keyboard_input.pressed(KeyCode::Left) {
        direction += Vec3::new(-1.0, 0.0, 0.0);
    }
    if keyboard_input.pressed(KeyCode::Right) {
        direction += Vec3::new(1.0, 0.0, 0.0);
    }
    if keyboard_input.pressed(KeyCode::Up) {
        direction += Vec3::new(0.0, 1.0, 0.0);
    }
    if keyboard_input.pressed(KeyCode::Down) {
        direction += Vec3::new(0.0, -1.0, 0.0);
    }

    if direction.length() > 0.0 {
        direction = direction.normalize();
    }

    player_transform.translation += direction * 200.0 * time.delta_seconds();

    // Clamp player position to game area
    player_transform.translation.x = player_transform.translation.x.clamp(
        -WINDOW_WIDTH * 0.45,
        WINDOW_WIDTH * 0.45,
    );
    player_transform.translation.y = player_transform.translation.y.clamp(
        -WINDOW_HEIGHT * 0.45,
        WINDOW_HEIGHT * 0.15,
    );
}

fn update_score(mut game_state: ResMut<GameState>, mut query: Query<&mut Text>) {
    // This is a placeholder for score update logic
    game_state.score += 1;
    if game_state.score > game_state.high_score {
        game_state.high_score = game_state.score;
    }

    for mut text in &mut query {
        if text.sections[0].value.starts_with("SCORE") {
            text.sections[0].value = format!("SCORE\n{}", game_state.score);
        } else if text.sections[0].value.starts_with("HIGH SCORE") {
            text.sections[0].value = format!("HIGH SCORE\n{}", game_state.high_score);
        }
    }
}
