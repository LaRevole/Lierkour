//! Процедурные векторные иконки интерфейса.
//!
//! Рисуются через `egui::Painter`, не зависят от внешних шрифтов
//! и никогда не вызывают артефактов («квадратов» / tofu-глифов).

use eframe::egui::{self, Color32, Pos2, Rect, Response, Stroke, Ui, Vec2};

/// Универсальная кнопка с векторной иконкой.
pub fn icon_button<F>(ui: &mut Ui, tooltip: &str, paint_fn: F) -> Response
where
    F: FnOnce(&egui::Painter, Rect, Color32),
{
    let size = Vec2::new(32.0, 28.0);
    let button = egui::Button::new("").min_size(size);
    let response = ui.add(button);
    let rect = response.rect;
    let visuals = ui.style().interact(&response);

    let icon_rect = Rect::from_center_size(rect.center(), Vec2::new(16.0, 16.0));
    let color = if response.hovered() {
        visuals.fg_stroke.color
    } else {
        visuals.fg_stroke.color.gamma_multiply(0.85)
    };
    paint_fn(ui.painter(), icon_rect, color);

    response.on_hover_text(tooltip)
}

/// Иконка шестерёнки (Настройки).
pub fn paint_gear(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let r_outer = 6.8;
    let r_inner = 2.8;

    // Зубья шестерёнки
    let stroke = Stroke::new(2.2, color);
    for i in 0..6 {
        let angle = i as f32 * std::f32::consts::PI / 3.0;
        let dir = Vec2::new(angle.cos(), angle.sin());
        painter.line_segment([center - dir * r_outer, center + dir * r_outer], stroke);
    }

    // Тело шестерёнки
    painter.circle_filled(center, 5.0, color);
    // Внутреннее отверстие
    painter.circle_stroke(center, r_inner, Stroke::new(1.4, color.gamma_multiply(0.3)));
}

/// Иконка диаграммы / аналитики (Статистика).
pub fn paint_chart(painter: &egui::Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(2.0, color);

    // Базовая линия
    painter.line_segment(
        [
            Pos2::new(rect.min.x, rect.max.y),
            Pos2::new(rect.max.x, rect.max.y),
        ],
        stroke,
    );

    // 3 вертикальных столбика
    let bar_w = 2.8;
    let col1_x = rect.min.x + 2.5;
    let col2_x = rect.min.x + 8.0;
    let col3_x = rect.min.x + 13.5;

    let baseline = rect.max.y - 1.0;

    // Столбик 1 (средний)
    painter.line_segment(
        [
            Pos2::new(col1_x, baseline),
            Pos2::new(col1_x, baseline - 8.0),
        ],
        Stroke::new(bar_w, color),
    );

    // Столбик 2 (высокий)
    painter.line_segment(
        [
            Pos2::new(col2_x, baseline),
            Pos2::new(col2_x, baseline - 14.0),
        ],
        Stroke::new(bar_w, color),
    );

    // Столбик 3 (средне-низкий)
    painter.line_segment(
        [
            Pos2::new(col3_x, baseline),
            Pos2::new(col3_x, baseline - 10.0),
        ],
        Stroke::new(bar_w, color),
    );
}

/// Иконка темы оформления (Солнце для светлой темы, полумесяц для тёмной).
pub fn paint_theme_sun_moon(
    painter: &egui::Painter,
    rect: Rect,
    is_currently_dark: bool,
    color: Color32,
) {
    let center = rect.center();

    if is_currently_dark {
        // Показываем солнце (чтобы переключиться на светлую тему)
        let r_center = 3.5;
        painter.circle_filled(center, r_center, color);

        let stroke = Stroke::new(1.5, color);
        for i in 0..8 {
            let angle = i as f32 * std::f32::consts::PI / 4.0;
            let dir = Vec2::new(angle.cos(), angle.sin());
            painter.line_segment([center + dir * 5.0, center + dir * 7.2], stroke);
        }
    } else {
        // Показываем полумесяц (чтобы переключиться на тёмную тему)
        let points = [
            center + Vec2::new(1.8, -6.0),
            center + Vec2::new(-3.5, -3.5),
            center + Vec2::new(-3.5, 3.5),
            center + Vec2::new(1.8, 6.0),
            center + Vec2::new(-0.8, 3.0),
            center + Vec2::new(-0.8, -3.0),
        ];
        painter.add(egui::Shape::convex_polygon(
            points.to_vec(),
            color,
            Stroke::NONE,
        ));
    }
}

/// Иконка импорта (стрелка вниз в контейнер).
#[allow(dead_code)]
pub fn paint_import(painter: &egui::Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.8, color);

    // Контейнер (лоток) снизу
    painter.line_segment(
        [
            Pos2::new(rect.min.x + 2.0, rect.max.y - 4.0),
            Pos2::new(rect.min.x + 2.0, rect.max.y),
        ],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(rect.min.x + 2.0, rect.max.y),
            Pos2::new(rect.max.x - 2.0, rect.max.y),
        ],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(rect.max.x - 2.0, rect.max.y),
            Pos2::new(rect.max.x - 2.0, rect.max.y - 4.0),
        ],
        stroke,
    );

    // Стрелка вниз
    let mid_x = rect.center().x;
    let arrow_tip = rect.max.y - 3.5;
    painter.line_segment(
        [
            Pos2::new(mid_x, rect.min.y + 1.0),
            Pos2::new(mid_x, arrow_tip),
        ],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(mid_x - 3.5, arrow_tip - 3.5),
            Pos2::new(mid_x, arrow_tip),
        ],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(mid_x + 3.5, arrow_tip - 3.5),
            Pos2::new(mid_x, arrow_tip),
        ],
        stroke,
    );
}

/// Иконка экспорта (стрелка вверх из контейнера).
#[allow(dead_code)]
pub fn paint_export(painter: &egui::Painter, rect: Rect, color: Color32) {
    let stroke = Stroke::new(1.8, color);

    // Контейнер снизу
    painter.line_segment(
        [
            Pos2::new(rect.min.x + 2.0, rect.max.y - 4.0),
            Pos2::new(rect.min.x + 2.0, rect.max.y),
        ],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(rect.min.x + 2.0, rect.max.y),
            Pos2::new(rect.max.x - 2.0, rect.max.y),
        ],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(rect.max.x - 2.0, rect.max.y),
            Pos2::new(rect.max.x - 2.0, rect.max.y - 4.0),
        ],
        stroke,
    );

    // Стрелка вверх
    let mid_x = rect.center().x;
    let arrow_tip = rect.min.y + 1.5;
    painter.line_segment(
        [
            Pos2::new(mid_x, rect.max.y - 3.0),
            Pos2::new(mid_x, arrow_tip),
        ],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(mid_x - 3.5, arrow_tip + 3.5),
            Pos2::new(mid_x, arrow_tip),
        ],
        stroke,
    );
    painter.line_segment(
        [
            Pos2::new(mid_x + 3.5, arrow_tip + 3.5),
            Pos2::new(mid_x, arrow_tip),
        ],
        stroke,
    );
}
