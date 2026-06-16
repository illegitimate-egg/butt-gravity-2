use glam::{DMat4, DQuat, DVec3, Mat4};

pub struct Camera {
    pub position: DVec3,

    /// Za warld orientation
    pub orientation: DQuat,

    /// Fov of y axis in radians
    pub fov_y: f64,

    pub near_plane: f64,
    pub far_plane: f64,
}

impl Camera {
    pub fn forward(&self) -> DVec3 {
        DMat4::from_quat(self.orientation).transform_vector3(-DVec3::Z)
    }

    pub fn right(&self) -> DVec3 {
        DMat4::from_quat(self.orientation).transform_vector3(DVec3::X)
    }

    pub fn up(&self) -> DVec3 {
        DMat4::from_quat(self.orientation).transform_vector3(DVec3::Y)
    }

    pub fn view_matrix(&self) -> DMat4 {
        let view_rotation = self.orientation.conjugate();
        let view_translation = view_rotation * -self.position;
        DMat4::from_rotation_translation(view_rotation, view_translation)
    }

    pub fn projection_matrix(&self, aspect_ratio: f64) -> DMat4 {
        DMat4::perspective_rh(self.fov_y, aspect_ratio, self.near_plane, self.far_plane)
    }

    pub fn view_projection_matrix(&self, aspect_ratio: f64) -> DMat4 {
        self.projection_matrix(aspect_ratio) * self.view_matrix()
    }

    pub fn view_projection_matrix_single(&self, aspect_ratio: f64) -> Mat4 {
        self.view_projection_matrix(aspect_ratio).as_mat4()
    }

    pub fn rotate_local(&mut self, rotation: DQuat) {
        self.orientation *= rotation;
        self.orientation = self.orientation.normalize();
    }

    pub fn rotate_world(&mut self, rotation: DQuat) {
        self.orientation = rotation * self.orientation;
        self.orientation = self.orientation.normalize();
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [f32; 16],
}
