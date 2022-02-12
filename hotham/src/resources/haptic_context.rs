use crate::components::hand::Handedness;

/// Wrapper around XR Haptics
#[derive(Clone, Debug, Default)]
pub struct HapticContext {
    /// Haptics that should be applied to the left hand
    pub left_hand_amplitude_this_frame: f32,
    /// Haptics that should be applied to the right hand
    pub right_hand_amplitude_this_frame: f32,
}

pub struct Haptic {
    pub hand: Handedness,
    pub amplitude_this_frame: f32,
}

impl HapticContext {
    /// Request haptics be applied this frame
    pub fn request_haptic_feedback(&mut self, amplitude: f32, handedness: Handedness) {
        match handedness {
            Handedness::Left => {
                if amplitude > self.left_hand_amplitude_this_frame {
                    self.left_hand_amplitude_this_frame = amplitude;
                }
            }
            Handedness::Right => {
                if amplitude > self.right_hand_amplitude_this_frame {
                    self.right_hand_amplitude_this_frame = amplitude;
                }
            }
        }
    }

    pub fn iter_mut(&mut self) -> std::vec::IntoIter<&mut f32> {
        vec![
            &mut self.left_hand_amplitude_this_frame,
            &mut self.right_hand_amplitude_this_frame,
        ]
        .into_iter()
    }
}
/*
impl IntoIterator for HapticContext {
    type Item = f32;
    type IntoIter = std::array::IntoIter<f32, 2>;

    fn into_iter(self) -> Self::IntoIter {
        std::array::IntoIter::new([
            self.left_hand_amplitude_this_frame,
            self.right_hand_amplitude_this_frame,
        ])
    }
}
*/
