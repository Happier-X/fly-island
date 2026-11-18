//! Recognizes two clean Ctrl press-and-release cycles.

const MAX_HOLD_MS: u32 = 250;
const MAX_GAP_MS: u32 = 300;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Key {
    LeftCtrl,
    RightCtrl,
    Other,
}

#[derive(Default)]
pub(super) struct CtrlTaps {
    held: u8,
    press: Option<u32>,
    first_release: Option<u32>,
}

impl CtrlTaps {
    pub(super) fn event(&mut self, key: Key, down: bool, time: u32, modifier_held: bool) -> bool {
        let bit = match key {
            Key::LeftCtrl => 1,
            Key::RightCtrl => 2,
            Key::Other => {
                if down || self.held != 0 {
                    self.cancel();
                }
                return false;
            }
        };
        if down {
            self.press = if self.held == 0 && !modifier_held {
                if self
                    .first_release
                    .is_some_and(|first| time.wrapping_sub(first) > MAX_GAP_MS)
                {
                    self.first_release = None;
                }
                Some(time)
            } else {
                self.cancel();
                None
            };
            self.held |= bit;
            return false;
        }
        if self.held & bit == 0 {
            self.cancel();
            return false;
        }
        self.held &= !bit;
        let clean = self
            .press
            .take()
            .is_some_and(|start| time.wrapping_sub(start) <= MAX_HOLD_MS)
            && self.held == 0
            && !modifier_held;
        if !clean {
            self.cancel();
            return false;
        }
        if self.first_release.take().is_some() {
            return true;
        }
        self.first_release = Some(time);
        false
    }

    fn cancel(&mut self) {
        self.press = None;
        self.first_release = None;
    }
}

#[cfg(test)]
mod tests {
    use super::{CtrlTaps, Key::*};

    fn events(events: &[(super::Key, bool, u32, bool)]) -> Vec<usize> {
        let mut taps = CtrlTaps::default();
        events
            .iter()
            .enumerate()
            .filter_map(|(i, &(key, down, time, modifier))| {
                taps.event(key, down, time, modifier).then_some(i)
            })
            .collect()
    }

    #[test]
    fn fires_once_at_the_second_release_and_accepts_either_ctrl() {
        assert_eq!(
            events(&[
                (LeftCtrl, true, 0, false),
                (LeftCtrl, false, 40, false),
                (RightCtrl, true, 110, false),
                (RightCtrl, false, 140, false),
                (LeftCtrl, true, 190, false),
                (LeftCtrl, false, 220, false),
            ]),
            [3]
        );
    }

    #[test]
    fn ctrl_c_cancels_either_tap() {
        let tap = [(LeftCtrl, true, 0, false), (LeftCtrl, false, 60, false)];
        let copy = [
            (LeftCtrl, true, 0, false),
            (Other, true, 20, false),
            (Other, false, 40, false),
            (LeftCtrl, false, 60, false),
        ];
        for (first, second) in [
            (tap.as_slice(), copy.as_slice()),
            (copy.as_slice(), tap.as_slice()),
        ] {
            let sequence = first
                .iter()
                .copied()
                .chain(second.iter().map(|&(k, d, t, m)| (k, d, t + 100, m)))
                .collect::<Vec<_>>();
            assert!(events(&sequence).is_empty());
        }
    }

    #[test]
    fn overlapping_ctrl_repeat_long_hold_and_existing_modifiers_cancel() {
        for bad in [
            vec![
                (LeftCtrl, true, 0, false),
                (RightCtrl, true, 10, false),
                (LeftCtrl, false, 20, false),
                (RightCtrl, false, 30, false),
            ],
            vec![
                (LeftCtrl, true, 0, false),
                (LeftCtrl, true, 20, false),
                (LeftCtrl, false, 40, false),
            ],
            vec![(LeftCtrl, true, 0, false), (LeftCtrl, false, 251, false)],
            vec![(LeftCtrl, true, 0, true), (LeftCtrl, false, 40, false)],
        ] {
            let mut sequence = bad;
            sequence.extend([(LeftCtrl, true, 270, false), (LeftCtrl, false, 310, false)]);
            assert!(events(&sequence).is_empty());
        }
    }

    #[test]
    fn timeout_and_intervening_keys_break_the_sequence() {
        assert_eq!(
            events(&[
                (LeftCtrl, true, 0, false),
                (LeftCtrl, false, 30, false),
                (LeftCtrl, true, 331, false),
                (LeftCtrl, false, 370, false),
                (Other, true, 380, false),
                (Other, false, 390, false),
                (LeftCtrl, true, 440, false),
                (LeftCtrl, false, 470, false),
                (LeftCtrl, true, 510, false),
                (LeftCtrl, false, 550, false),
            ]),
            [9]
        );
    }

    #[test]
    fn windows_message_ticks_can_wrap() {
        assert_eq!(
            events(&[
                (LeftCtrl, true, u32::MAX - 60, false),
                (LeftCtrl, false, u32::MAX - 20, false),
                (RightCtrl, true, 15, false),
                (RightCtrl, false, 40, false),
            ]),
            [3]
        );
    }
}
