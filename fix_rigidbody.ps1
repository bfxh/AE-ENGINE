$file = "D:\rj\wasteland_project\wasteland_physics\src\world.rs"
$content = [System.IO.File]::ReadAllText($file)

# Step 1: Add mpss_index field to RigidBody
$old1 = @'
    pub linear_damping: FixedPoint,
    pub angular_damping: FixedPoint,
}
'@
$new1 = @'
    pub linear_damping: FixedPoint,
    pub angular_damping: FixedPoint,
    pub mpss_index: Option<usize>, // Phase 6: unified particle field index
}
'@
$content = $content.Replace($old1, $new1)

# Step 2: Add sync_to_mpss method before serialize_state
$old2 = '    pub fn serialize_state(&self) -> Vec<u8> {'
$new2 = @'
    /// Phase 6: Sync rigid body states to MpssBuffer
    pub fn sync_to_mpss(&mut self, mpss: &mut wasteland_particle::mpss::MpssBuffer) {
        for body in &mut self.rigid_bodies {
            if body.body_type != BodyType::Dynamic {
                continue;
            }

            let pos = [body.position.x.to_f32(), body.position.y.to_f32(), body.position.z.to_f32()];
            let vel = [body.velocity.x.to_f32(), body.velocity.y.to_f32(), body.velocity.z.to_f32()];
            let temp = self.temperature.to_f32();
            let mass = body.mass.to_f32();

            if let Some(idx) = body.mpss_index {
                if idx < mpss.capacity && mpss.active[idx] {
                    mpss.pos[idx] = pos;
                    mpss.vel[idx] = vel;
                    mpss.temperature[idx] = temp;
                    mpss.mass[idx] = mass;
                } else {
                    body.mpss_index = mpss.spawn().map(|idx| {
                        mpss.pos[idx] = pos;
                        mpss.vel[idx] = vel;
                        mpss.temperature[idx] = temp;
                        mpss.mass[idx] = mass;
                        mpss.material_idx[idx] = 0;
                        mpss.lifetime[idx] = f32::MAX;
                        idx
                    });
                }
            } else {
                body.mpss_index = mpss.spawn().map(|idx| {
                    mpss.pos[idx] = pos;
                    mpss.vel[idx] = vel;
                    mpss.temperature[idx] = temp;
                    mpss.mass[idx] = mass;
                    mpss.material_idx[idx] = 0;
                    mpss.lifetime[idx] = f32::MAX;
                    idx
                });
            }
        }
    }

    /// Phase 6: Sync back from MpssBuffer to rigid bodies
    pub fn sync_from_mpss(&mut self, mpss: &wasteland_particle::mpss::MpssBuffer) {
        for body in &mut self.rigid_bodies {
            if let Some(idx) = body.mpss_index {
                if idx < mpss.capacity && mpss.active[idx] {
                    body.position = FixedVec3::from_f32(mpss.pos[idx][0], mpss.pos[idx][1], mpss.pos[idx][2]);
                    body.velocity = FixedVec3::from_f32(mpss.vel[idx][0], mpss.vel[idx][1], mpss.vel[idx][2]);
                }
            }
        }
    }

    pub fn serialize_state(&self) -> Vec<u8> {
'@
$content = $content.Replace($old2, $new2)

[System.IO.File]::WriteAllText($file, $content)
Write-Output "Done"
