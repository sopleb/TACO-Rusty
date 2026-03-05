use glam::{Mat4, Vec3, Vec4};

pub fn unproject(
    screen_x: f32,
    screen_y: f32,
    viewport_w: f32,
    viewport_h: f32,
    projection: &Mat4,
    modelview: &Mat4,
) -> (Vec3, Vec3) {
    let inv = (*projection * *modelview).inverse();

    let nx = (2.0 * screen_x / viewport_w) - 1.0;
    let ny = 1.0 - (2.0 * screen_y / viewport_h);

    let near = inv * Vec4::new(nx, ny, -1.0, 1.0);
    let far = inv * Vec4::new(nx, ny, 1.0, 1.0);

    let near = Vec3::new(near.x / near.w, near.y / near.w, near.z / near.w);
    let far = Vec3::new(far.x / far.w, far.y / far.w, far.z / far.w);

    let direction = (far - near).normalize();
    (near, direction)
}

pub fn ray_sphere_intersect(
    ray_origin: Vec3,
    ray_dir: Vec3,
    sphere_center: Vec3,
    sphere_radius: f32,
) -> bool {
    let oc = ray_origin - sphere_center;
    let a = ray_dir.dot(ray_dir);
    let b = 2.0 * oc.dot(ray_dir);
    let c = oc.dot(oc) - sphere_radius * sphere_radius;
    let discriminant = b * b - 4.0 * a * c;
    discriminant >= 0.0
}

