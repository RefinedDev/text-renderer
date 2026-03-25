use core::f32;

use bevy::{
    color::palettes::css::GREEN, 
    ecs::{query::With, system::Single}, 
    gizmos::gizmos::Gizmos, math::{Vec2, ops::{cos, sin}}, 
    render::camera::Camera, 
    transform::components::GlobalTransform
};

// Custom bounded box, with its own text
#[derive(Default)]
pub struct TextFrame {
    pub name: String,
    size: Vec2,     // 0 < x,y < 1 (we make the bounding box with this)
    position: Vec2, // 0 < x,y < 1 (this will be the center of the frame)
    
    pub text: String,
    pub angle: f32,
    pub frame_scale: f32,
    pub locked: bool, // frame remains on screen

    // pub center: Vec2,
    pub t_left: Vec2,
    pub t_right: Vec2,
    pub b_left: Vec2,
    pub b_right: Vec2,
}

pub trait Frame {
    fn get_frame_by_name(&mut self, name: String) -> Option<&mut TextFrame>;
}
impl Frame for Vec<TextFrame> {
    fn get_frame_by_name(&mut self, name: String) -> Option<&mut TextFrame> {
       self.iter_mut().find(|frame| frame.name == name)
    }
}

fn rotate_vector(v: &mut Vec2, angle: f32) {
    let rotated_x = v.x * cos(angle) + v.y * sin(angle);
    let rotated_y = -v.x * sin(angle) + v.y * cos(angle);
    v.x = rotated_x;
    v.y = rotated_y;
}

impl TextFrame {
    pub fn new(name: String, text: String, size: Vec2, position: Vec2, locked: bool, mut frame_scale: Option<f32>) -> Self {
        Self {
            name,
            text,
            size,
            position,
            locked,
            frame_scale: *frame_scale.get_or_insert(size.x),
            ..Default::default()
        }
    }

    pub fn setup_bounds(
        mut self,
        screen_dimensions: Vec2,
        camera: &Single<(&GlobalTransform, &Camera), With<Camera>>,
    ) -> Self {
        self.update(screen_dimensions, camera.0, camera.1);
        self
    }
    
    pub fn update(
        &mut self,
        screen_dimensions: Vec2,
        camera_transform: &GlobalTransform,
        camera: &Camera,
    ) {
        let (s_w, s_h) = (screen_dimensions.x, screen_dimensions.y);
        let position = camera.viewport_to_world_2d(camera_transform,Vec2::new(self.position.x * s_w, self.position.y * s_h)).unwrap();

        let width = s_w * self.size.x * 0.5;
        let height = s_h * self.size.y * 0.5;

        // self.center = Vec2::new(position.x, position.y);
        self.t_left = Vec2::new(position.x - width, position.y + height);
        self.t_right = Vec2::new(position.x + width, position.y + height);
        self.b_left = Vec2::new(position.x - width, position.y - height);
        self.b_right = Vec2::new(position.x + width, position.y - height);
        // rotate_vector(&mut self.t_left, self.angle);
        // rotate_vector(&mut self.t_right, self.angle);
        // rotate_vector(&mut self.b_left, self.angle);
        // rotate_vector(&mut self.b_right, self.angle);
    }

    pub fn show(&self, gizmos: &mut Gizmos) {
        gizmos.line_2d(self.t_left, self.t_right, GREEN);
        gizmos.line_2d(self.t_left, self.b_left, GREEN);
        gizmos.line_2d(self.t_right, self.b_right, GREEN);
        gizmos.line_2d(self.b_left, self.b_right, GREEN);
        // gizmos.circle_2d(self.center, 10.0, GREEN);
    }
}
