//! Simple perspective camera for the S-03 software rasterizer.

/// A pinhole perspective camera orbiting a target point (the chunk center).
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    /// Yaw in degrees (rotation around the vertical Y axis).
    pub yaw_deg: f32,
    /// Pitch in degrees (elevation; clamped away from the poles).
    pub pitch_deg: f32,
    /// Distance from the target to the camera eye, in world units.
    pub distance: f32,
    /// Vertical field of view in degrees.
    pub fov_deg: f32,
}

impl Camera {
    /// Construct a camera. `distance` is the eye-to-target distance; `fov_deg` the vertical FOV.
    pub fn new(yaw_deg: f32, pitch_deg: f32, distance: f32, fov_deg: f32) -> Self {
        let pitch = pitch_deg.clamp(-89.0, 89.0);
        Self {
            yaw_deg,
            pitch_deg: pitch,
            distance,
            fov_deg,
        }
    }

    /// The world-space eye position, orbiting `target` at `distance`.
    pub fn eye(&self, target: [f32; 3]) -> [f32; 3] {
        let yaw = self.yaw_deg.to_radians();
        let pitch = self.pitch_deg.to_radians();
        let x = target[0] + self.distance * pitch.cos() * yaw.cos();
        let y = target[1] + self.distance * pitch.sin();
        let z = target[2] + self.distance * pitch.cos() * yaw.sin();
        [x, y, z]
    }
}
