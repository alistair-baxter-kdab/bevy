//! A material that uses bindless textures.

use bevy::diagnostic::LogDiagnosticsPlugin;
use bevy::pbr::diagnostic::MaterialAllocatorDiagnosticPlugin;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

const SHADER_ASSET_PATH: &str = "shaders/bindless_material.wesl";

// `#[bindless(limit(4))]` indicates that we want Bevy to group materials into
// bind groups of at most 4 materials each.
// Note that we use the structure-level `#[uniform]` attribute to supply
// ordinary data to the shader.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[uniform(0, BindlessMaterialUniform, binding_array(10))]
#[bindless(limit(4))]
struct BindlessMaterial {
    color: LinearRgba,
    // This will be exposed to the shader as a binding array of 4 textures and a
    // binding array of 4 samplers.
    #[texture(1)]
    #[sampler(2)]
    color_texture: Option<Handle<Image>>,
    // To reproduce the allocated resource count drift, this is set to the same
    // image as `color_texture`.
    #[texture(3)]
    #[sampler(4)]
    second_texture: Option<Handle<Image>>,
}

// This buffer will be presented to the shader as `@binding(10)`.
#[derive(ShaderType)]
struct BindlessMaterialUniform {
    color: LinearRgba,
}

impl<'a> From<&'a BindlessMaterial> for BindlessMaterialUniform {
    fn from(material: &'a BindlessMaterial) -> Self {
        BindlessMaterialUniform {
            color: material.color,
        }
    }
}

// The entry point.
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MaterialPlugin::<BindlessMaterial>::default(),
            MaterialAllocatorDiagnosticPlugin::<BindlessMaterial>::default(),
            LogDiagnosticsPlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, swap_textures)
        .run();
}

#[derive(Resource)]
struct SwapImages {
    a: Handle<Image>,
    b: Handle<Image>,
}

// Every frame, swap both texture fields of every material between images A
// and B, keeping them pointing at the same image. With the bug present,
// `material_allocator_allocations` grows every frame instead of staying at the
// real resource count, and `material_allocator_slabs` climbs once a slab fills.
fn swap_textures(
    images: Res<SwapImages>,
    query: Query<&MeshMaterial3d<BindlessMaterial>>,
    mut materials: ResMut<Assets<BindlessMaterial>>,
) {
    for mesh_material in &query {
        if let Some(mut material) = materials.get_mut(&mesh_material.0) {
            let next = if material.color_texture.as_ref() == Some(&images.a) {
                images.b.clone()
            } else {
                images.a.clone()
            };
            material.color_texture = Some(next.clone());
            material.second_texture = Some(next);
        }
    }
}

// Creates a simple scene.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BindlessMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let a: Handle<Image> = asset_server.load("branding/bevy_logo_dark.png");
    let b: Handle<Image> = asset_server.load("branding/bevy_logo_light.png");
    commands.insert_resource(SwapImages {
        a: a.clone(),
        b: b.clone(),
    });

    // Add a cube with a blue tinted texture.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(BindlessMaterial {
            color: LinearRgba::BLUE,
            color_texture: Some(a.clone()),
            second_texture: Some(a),
        })),
        Transform::from_xyz(-2.0, 0.5, 0.0),
    ));

    // Add a cylinder with a red tinted texture.
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::default())),
        MeshMaterial3d(materials.add(BindlessMaterial {
            color: LinearRgba::RED,
            color_texture: Some(b.clone()),
            second_texture: Some(b),
        })),
        Transform::from_xyz(2.0, 0.5, 0.0),
    ));

    // Add a camera.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

impl Material for BindlessMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}
