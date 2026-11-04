//! Распознавание жестов навигации «назад/вперёд».
//!
//! Поддерживаются:
//! * боковые кнопки мыши (`PointerButton::Extra1` / `Extra2`);
//! * горизонтальный жест двумя пальцами по тачпаду.
//!
//! Для тачпада события приходят как обычные `MouseWheel`. Жест считается
//! горизонтальным, только если смещение по X больше смещения по Y. Чтобы
//! один жест не переключал несколько страниц подряд, накопление сбрасывается
//! после срабатывания и на короткое время включается «задержка».

use eframe::egui::{Event, MouseWheelSource, MouseWheelUnit, PointerButton, Vec2};

/// Насколько сильно нужно провести по тачпаду (в точках), чтобы сработала навигация.
const SWIPE_THRESHOLD: f32 = 90.0;

/// Пауза между событиями, после которой начинается новый жест.
const GESTURE_TIMEOUT_SECONDS: f64 = 0.30;

/// Пауза после срабатывания, чтобы один жест не переключал страницы несколько раз.
const COOLDOWN_SECONDS: f64 = 0.45;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Back,
    Forward,
}

#[derive(Default)]
pub struct Detector {
    /// Накопленное горизонтальное смещение текущего жеста (в точках).
    accumulated_x: f32,
    /// Время последнего горизонтального события.
    last_event_time: f64,
    /// До какого времени новые жесты игнорируются.
    cooldown_until: f64,
}

impl Detector {
    /// Возвращает направление навигации, если в событиях этого кадра
    /// был боковой клик мыши или завершился горизонтальный жест.
    ///
    /// `viewport_height` и `line_scroll_speed` нужны, чтобы привести
    /// строки/страницы прокрутки к точкам.
    pub fn poll(
        &mut self,
        events: &[Event],
        time: f64,
        viewport_height: f32,
        line_scroll_speed: f32,
        enabled: bool,
    ) -> Option<Direction> {
        if !enabled {
            self.reset_gesture();
            return None;
        }

        for event in events {
            match event {
                Event::PointerButton {
                    button,
                    pressed: true,
                    ..
                } => match button {
                    PointerButton::Extra1 => return Some(Direction::Back),
                    PointerButton::Extra2 => return Some(Direction::Forward),
                    _ => {}
                },
                Event::MouseWheel {
                    unit,
                    delta,
                    source,
                    modifiers,
                    ..
                } => {
                    // Инерция после жеста и прокрутка с зажатыми модификаторами
                    // (например, Ctrl+колесо для масштаба) навигацией не считаются.
                    if *source == MouseWheelSource::Momentum || modifiers.any() {
                        continue;
                    }

                    let delta =
                        wheel_delta_points(*unit, *delta, viewport_height, line_scroll_speed);
                    if delta.x.abs() <= delta.y.abs() {
                        continue;
                    }

                    if time - self.last_event_time > GESTURE_TIMEOUT_SECONDS {
                        self.accumulated_x = 0.0;
                    }
                    self.last_event_time = time;

                    if time < self.cooldown_until {
                        self.accumulated_x = 0.0;
                        continue;
                    }

                    // Смена направления — это уже другой жест.
                    if self.accumulated_x != 0.0 && self.accumulated_x.signum() != delta.x.signum()
                    {
                        self.accumulated_x = 0.0;
                    }

                    self.accumulated_x += delta.x;
                    if self.accumulated_x.abs() >= SWIPE_THRESHOLD {
                        // Положительное смещение — контент движется вправо,
                        // то есть два пальца ведут вправо → «назад».
                        let direction = if self.accumulated_x > 0.0 {
                            Direction::Back
                        } else {
                            Direction::Forward
                        };
                        self.accumulated_x = 0.0;
                        self.cooldown_until = time + COOLDOWN_SECONDS;
                        return Some(direction);
                    }
                }
                _ => {}
            }
        }

        None
    }

    fn reset_gesture(&mut self) {
        self.accumulated_x = 0.0;
    }
}

fn wheel_delta_points(
    unit: MouseWheelUnit,
    delta: Vec2,
    viewport_height: f32,
    line_scroll_speed: f32,
) -> Vec2 {
    match unit {
        MouseWheelUnit::Point => delta,
        MouseWheelUnit::Line => delta * line_scroll_speed,
        MouseWheelUnit::Page => delta * viewport_height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Modifiers, Pos2, TouchPhase};

    fn wheel(delta: Vec2, unit: MouseWheelUnit, source: MouseWheelSource) -> Event {
        Event::MouseWheel {
            unit,
            delta,
            phase: TouchPhase::Move,
            source,
            modifiers: Modifiers::default(),
        }
    }

    fn swipe(delta_x: f32) -> Event {
        wheel(
            Vec2::new(delta_x, 0.0),
            MouseWheelUnit::Point,
            MouseWheelSource::Trackpad,
        )
    }

    fn button(button: PointerButton, pressed: bool) -> Event {
        Event::PointerButton {
            pos: Pos2::ZERO,
            button,
            pressed,
            modifiers: Modifiers::default(),
        }
    }

    fn poll(detector: &mut Detector, events: &[Event], time: f64) -> Option<Direction> {
        detector.poll(events, time, 800.0, 40.0, true)
    }

    #[test]
    fn back_button_goes_back() {
        let mut detector = Detector::default();
        assert_eq!(
            poll(&mut detector, &[button(PointerButton::Extra1, true)], 1.0),
            Some(Direction::Back)
        );
    }

    #[test]
    fn forward_button_goes_forward() {
        let mut detector = Detector::default();
        assert_eq!(
            poll(&mut detector, &[button(PointerButton::Extra2, true)], 1.0),
            Some(Direction::Forward)
        );
    }

    #[test]
    fn button_release_is_ignored() {
        let mut detector = Detector::default();
        assert_eq!(
            poll(&mut detector, &[button(PointerButton::Extra1, false)], 1.0),
            None
        );
    }

    #[test]
    fn main_buttons_are_ignored() {
        let mut detector = Detector::default();
        assert_eq!(
            poll(&mut detector, &[button(PointerButton::Primary, true)], 1.0),
            None
        );
    }

    #[test]
    fn swipe_right_accumulates_into_back() {
        let mut detector = Detector::default();
        assert_eq!(poll(&mut detector, &[swipe(50.0)], 1.0), None);
        assert_eq!(
            poll(&mut detector, &[swipe(50.0)], 1.1),
            Some(Direction::Back)
        );
    }

    #[test]
    fn swipe_left_goes_forward() {
        let mut detector = Detector::default();
        assert_eq!(
            poll(&mut detector, &[swipe(-100.0)], 1.0),
            Some(Direction::Forward)
        );
    }

    #[test]
    fn vertical_scroll_is_ignored() {
        let mut detector = Detector::default();
        let event = wheel(
            Vec2::new(10.0, 200.0),
            MouseWheelUnit::Point,
            MouseWheelSource::Trackpad,
        );
        assert_eq!(poll(&mut detector, std::slice::from_ref(&event), 1.0), None);
        assert_eq!(poll(&mut detector, &[event], 1.1), None);
    }

    #[test]
    fn modified_scroll_is_ignored() {
        let mut detector = Detector::default();
        let event = Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: Vec2::new(500.0, 0.0),
            phase: TouchPhase::Move,
            source: MouseWheelSource::Trackpad,
            modifiers: Modifiers {
                shift: true,
                ..Default::default()
            },
        };
        assert_eq!(poll(&mut detector, &[event], 1.0), None);
    }

    #[test]
    fn momentum_is_ignored() {
        let mut detector = Detector::default();
        let event = wheel(
            Vec2::new(500.0, 0.0),
            MouseWheelUnit::Point,
            MouseWheelSource::Momentum,
        );
        assert_eq!(poll(&mut detector, &[event], 1.0), None);
    }

    #[test]
    fn direction_reversal_starts_new_gesture() {
        let mut detector = Detector::default();
        assert_eq!(poll(&mut detector, &[swipe(60.0)], 1.0), None);
        assert_eq!(poll(&mut detector, &[swipe(-60.0)], 1.1), None);
        assert_eq!(
            poll(&mut detector, &[swipe(-40.0)], 1.2),
            Some(Direction::Forward)
        );
    }

    #[test]
    fn cooldown_blocks_repeated_navigation() {
        let mut detector = Detector::default();
        assert_eq!(
            poll(&mut detector, &[swipe(100.0)], 1.0),
            Some(Direction::Back)
        );
        assert_eq!(poll(&mut detector, &[swipe(100.0)], 1.1), None);
        assert_eq!(
            poll(&mut detector, &[swipe(100.0)], 1.6),
            Some(Direction::Back)
        );
    }

    #[test]
    fn long_pause_resets_accumulation() {
        let mut detector = Detector::default();
        assert_eq!(poll(&mut detector, &[swipe(60.0)], 1.0), None);
        assert_eq!(poll(&mut detector, &[swipe(60.0)], 2.0), None);
    }

    #[test]
    fn line_scroll_is_scaled_to_points() {
        let mut detector = Detector::default();
        let two_lines = wheel(
            Vec2::new(2.0, 0.0),
            MouseWheelUnit::Line,
            MouseWheelSource::Unknown,
        );
        assert_eq!(poll(&mut detector, &[two_lines], 1.0), None);

        let third_line = wheel(
            Vec2::new(1.0, 0.0),
            MouseWheelUnit::Line,
            MouseWheelSource::Unknown,
        );
        assert_eq!(
            poll(&mut detector, &[third_line], 1.1),
            Some(Direction::Back)
        );
    }

    #[test]
    fn disabled_detector_ignores_everything() {
        let mut detector = Detector::default();
        let events = [button(PointerButton::Extra1, true), swipe(500.0)];
        assert_eq!(detector.poll(&events, 1.0, 800.0, 40.0, false), None);
    }
}
