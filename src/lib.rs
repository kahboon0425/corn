use avian3d::prelude::*;
use bevy::color::palettes::tailwind::{PINK_100, RED_500};
use bevy::core_pipeline::Skybox;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::smaa::Smaa;
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::picking::pointer::PointerInteraction;
use bevy::prelude::*;
use bevy::scene::SceneInstanceReady;

pub struct CornPlugin;

impl Plugin for CornPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PhysicsPlugins::default(),
            PhysicsPickingPlugin,
            PhysicsDebugPlugin::default(),
            bevy_skein::SkeinPlugin::default(),
            bevy_panorbit_camera::PanOrbitCameraPlugin,
        ))
        .add_systems(
            Startup,
            (setup_camera_and_environment, setup_mesh_and_animation),
        )
        .add_systems(Update, draw_mesh_intersections);

        #[cfg(feature = "dev")]
        app.add_plugins((
            bevy_inspector_egui::bevy_egui::EguiPlugin {
                enable_multipass_for_primary_context: true,
            },
            bevy_inspector_egui::quick::WorldInspectorPlugin::new(),
        ));
    }
}

const FACTORY: &str = "factory.glb";
const CORN: &str = "corn.glb";

fn setup_mesh_and_animation(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    // Create an animation graph containing a single animation. We want the "run"
    // animation from our example asset, which has an index of two.
    let (graph, index) = AnimationGraph::from_clip(
        asset_server
            .load(GltfAssetLabel::Animation(0).from_asset(FACTORY)),
    );

    // Store the animation graph as an asset.
    let graph_handle = graphs.add(graph);

    // Create a component that stores a reference to our animation.
    let animation_to_play = AnimationToPlay {
        graph_handle,
        index,
    };

    // Start loading the asset as a scene and store a reference to it in a
    // SceneRoot component. This component will automatically spawn a scene
    // containing our mesh once it has loaded.
    let mesh_scene = SceneRoot(
        asset_server
            .load(GltfAssetLabel::Scene(0).from_asset(FACTORY)),
    );

    // Spawn an entity with our components, and connect it to an observer that
    // will trigger when the scene is loaded and spawned.
    commands
        .spawn((animation_to_play, mesh_scene))
        .observe(play_animation_when_ready);

    commands.spawn((
        SceneRoot(
            asset_server
                .load(GltfAssetLabel::Scene(0).from_asset(CORN)),
        ),
        Transform::from_xyz(5.0, 10.0, 5.0).with_rotation(
            Quat::from_euler(EulerRot::XYZ, 3.57, 0.14, 2.95),
        ),
    ));
}

fn setup_camera_and_environment(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // Camera.
    const INITIAL_FOCUS: Vec3 = Vec3::new(0.0, 3.0, 0.0);

    commands.spawn((
        Camera3d::default(),
        Camera {
            hdr: true,
            ..default()
        },
        Tonemapping::BlenderFilmic,
        Bloom::NATURAL,
        Transform::from_xyz(-3.5, 10.0, 15.0)
            .looking_at(INITIAL_FOCUS, Vec3::Y),
        DebandDither::Enabled,
        bevy_panorbit_camera::PanOrbitCamera {
            focus: INITIAL_FOCUS,
            zoom_sensitivity: 0.5,
            button_pan: MouseButton::Middle,
            ..default()
        },
        Msaa::Off,
        ScreenSpaceAmbientOcclusion::default(),
        Smaa::default(),
        Skybox {
            image: asset_server.load("pisa_diffuse_rgb9e5_zstd.ktx2"),
            brightness: 1000.0,
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server
                .load("pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server
                .load("pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 1000.0,
            ..default()
        },
    ));
}

fn play_animation_when_ready(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    animations_to_play: Query<&AnimationToPlay>,
    mut players: Query<&mut AnimationPlayer>,
) {
    // The entity we spawned in `setup_mesh_and_animation` is the trigger's target.
    // Start by finding the AnimationToPlay component we added to that entity.
    if let Ok(animation_to_play) =
        animations_to_play.get(trigger.target())
    {
        // The SceneRoot component will have spawned the scene as a hierarchy
        // of entities parented to our entity. Since the asset contained a skinned
        // mesh and animations, it will also have spawned an animation player
        // component. Search our entity's descendants to find the animation player.
        for child in children.iter_descendants(trigger.target()) {
            if let Ok(mut player) = players.get_mut(child) {
                // Tell the animation player to start the animation and keep
                // repeating it.
                //
                // If you want to try stopping and switching animations, see the
                // `animated_mesh_control.rs` example.
                player.play(animation_to_play.index).repeat();

                // Add the animation graph. This only needs to be done once to
                // connect the animation player to the mesh.
                commands.entity(child).insert(AnimationGraphHandle(
                    animation_to_play.graph_handle.clone(),
                ));
            }
        }
    }
}

/// A system that draws hit indicators for every pointer.
fn draw_mesh_intersections(
    q_pointers: Query<&PointerInteraction>,
    mut gizmos: Gizmos,
) {
    for (point, normal) in q_pointers
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .filter_map(|(_entity, hit)| hit.position.zip(hit.normal))
    {
        gizmos.sphere(point, 0.05, RED_500);
        gizmos.arrow(
            point,
            point + normal.normalize() * 0.5,
            PINK_100,
        );
    }
}

// A component that stores a reference to an animation we want to play. This is
// created when we start loading the mesh (see `setup_mesh_and_animation`) and
// read when the mesh has spawned (see `play_animation_once_loaded`).
#[derive(Component)]
struct AnimationToPlay {
    graph_handle: Handle<AnimationGraph>,
    index: AnimationNodeIndex,
}
