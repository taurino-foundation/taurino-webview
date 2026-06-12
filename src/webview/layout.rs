use dpi::{LogicalPosition, LogicalSize};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl LayoutBounds {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn full(width: f32, height: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width,
            height,
        }
    }

    pub fn to_wry_rect(self) -> wry::Rect {
        wry::Rect {
            position: LogicalPosition::new(self.x, self.y).into(),
            size: LogicalSize::new(self.width, self.height).into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FixedLayout {
    Single,

    HorizontalSplit,

    VerticalSplit,

    Grid {
        rows: usize,
        cols: usize,
    },

    SidebarLeft {
        width: f32,
    },

    SidebarRight {
        width: f32,
    },

    HeaderBody {
        header_height: f32,
    },

    BodyFooter {
        footer_height: f32,
    },

    HeaderBodyFooter {
        header_height: f32,
        footer_height: f32,
    },

    Docked {
        left_width: Option<f32>,
        right_width: Option<f32>,
        header_height: Option<f32>,
        footer_height: Option<f32>,
    },

    Overlay {
        overlay: LayoutBounds,
    },

    CustomAbsolute {
        bounds: Vec<LayoutBounds>,
    },
}

impl FixedLayout {
    pub fn resolve(
        &self,
        count: usize,
        window_width: f32,
        window_height: f32,
    ) -> Vec<LayoutBounds> {
        if count == 0 {
            return Vec::new();
        }

        let full = LayoutBounds::full(window_width, window_height);

        match self {
            FixedLayout::Single => {
                vec![full; count]
            }

            FixedLayout::HorizontalSplit => split_horizontal(full, count),

            FixedLayout::VerticalSplit => split_vertical(full, count),

            FixedLayout::Grid { rows, cols } => grid(full, count, *rows, *cols),

            FixedLayout::SidebarLeft { width } => {
                if count == 1 {
                    return vec![full];
                }

                let sidebar_width = width.min(window_width);
                let main_width = (window_width - sidebar_width).max(0.0);

                let sidebar =
                    LayoutBounds::new(0.0, 0.0, sidebar_width, window_height);

                let main = LayoutBounds::new(
                    sidebar_width,
                    0.0,
                    main_width,
                    window_height,
                );

                let mut result = vec![sidebar];

                if count == 2 {
                    result.push(main);
                } else {
                    result.extend(split_vertical(main, count - 1));
                }

                result
            }

            FixedLayout::SidebarRight { width } => {
                if count == 1 {
                    return vec![full];
                }

                let sidebar_width = width.min(window_width);
                let main_width = (window_width - sidebar_width).max(0.0);

                let main =
                    LayoutBounds::new(0.0, 0.0, main_width, window_height);

                let sidebar = LayoutBounds::new(
                    main_width,
                    0.0,
                    sidebar_width,
                    window_height,
                );

                let mut result = vec![main];

                if count == 2 {
                    result.push(sidebar);
                } else {
                    result.extend(split_vertical(sidebar, count - 1));
                }

                result
            }

            FixedLayout::HeaderBody { header_height } => {
                if count == 1 {
                    return vec![full];
                }

                let header_height = header_height.min(window_height);
                let body_height = (window_height - header_height).max(0.0);

                let header =
                    LayoutBounds::new(0.0, 0.0, window_width, header_height);

                let body = LayoutBounds::new(
                    0.0,
                    header_height,
                    window_width,
                    body_height,
                );

                let mut result = vec![header];
                result.extend(split_horizontal(body, count - 1));
                result
            }

            FixedLayout::BodyFooter { footer_height } => {
                if count == 1 {
                    return vec![full];
                }

                let footer_height = footer_height.min(window_height);
                let body_height = (window_height - footer_height).max(0.0);

                let body =
                    LayoutBounds::new(0.0, 0.0, window_width, body_height);

                let footer = LayoutBounds::new(
                    0.0,
                    body_height,
                    window_width,
                    footer_height,
                );

                let mut result = split_horizontal(body, count - 1);
                result.push(footer);
                result
            }

            FixedLayout::HeaderBodyFooter {
                header_height,
                footer_height,
            } => {
                if count == 1 {
                    return vec![full];
                }

                let header_height = header_height.min(window_height);
                let footer_height =
                    footer_height.min(window_height - header_height);

                let body_height =
                    (window_height - header_height - footer_height).max(0.0);

                let header =
                    LayoutBounds::new(0.0, 0.0, window_width, header_height);

                let body = LayoutBounds::new(
                    0.0,
                    header_height,
                    window_width,
                    body_height,
                );

                let footer = LayoutBounds::new(
                    0.0,
                    header_height + body_height,
                    window_width,
                    footer_height,
                );

                let mut result = Vec::new();
                result.push(header);

                if count > 2 {
                    result.extend(split_horizontal(body, count - 2));
                    result.push(footer);
                } else {
                    result.push(body);
                }

                result
            }

            FixedLayout::Docked {
                left_width,
                right_width,
                header_height,
                footer_height,
            } => {
                let header_h = header_height.unwrap_or(0.0).min(window_height);

                let footer_h =
                    footer_height.unwrap_or(0.0).min(window_height - header_h);

                let center_y = header_h;
                let center_h = (window_height - header_h - footer_h).max(0.0);

                let left_w = left_width.unwrap_or(0.0).min(window_width);

                let right_w =
                    right_width.unwrap_or(0.0).min(window_width - left_w);

                let center_x = left_w;
                let center_w = (window_width - left_w - right_w).max(0.0);

                let mut result = Vec::new();

                if header_height.is_some() {
                    result.push(LayoutBounds::new(
                        0.0,
                        0.0,
                        window_width,
                        header_h,
                    ));
                }

                if left_width.is_some() {
                    result.push(LayoutBounds::new(
                        0.0, center_y, left_w, center_h,
                    ));
                }

                result.push(LayoutBounds::new(
                    center_x, center_y, center_w, center_h,
                ));

                if right_width.is_some() {
                    result.push(LayoutBounds::new(
                        center_x + center_w,
                        center_y,
                        right_w,
                        center_h,
                    ));
                }

                if footer_height.is_some() {
                    result.push(LayoutBounds::new(
                        0.0,
                        center_y + center_h,
                        window_width,
                        footer_h,
                    ));
                }

                while result.len() < count {
                    result.push(LayoutBounds::new(
                        center_x, center_y, center_w, center_h,
                    ));
                }

                result.truncate(count);
                result
            }

            FixedLayout::Overlay { overlay } => {
                let mut result = Vec::new();
                result.push(full);

                for _ in 1..count {
                    result.push(*overlay);
                }

                result
            }

            FixedLayout::CustomAbsolute { bounds } => {
                let mut result = bounds.clone();

                while result.len() < count {
                    result.push(full);
                }

                result.truncate(count);
                result
            }
        }
    }
}

fn split_horizontal(area: LayoutBounds, count: usize) -> Vec<LayoutBounds> {
    if count == 0 {
        return Vec::new();
    }

    let width = area.width / count as f32;

    (0..count)
        .map(|index| {
            LayoutBounds::new(
                area.x + width * index as f32,
                area.y,
                width,
                area.height,
            )
        })
        .collect()
}

fn split_vertical(area: LayoutBounds, count: usize) -> Vec<LayoutBounds> {
    if count == 0 {
        return Vec::new();
    }

    let height = area.height / count as f32;

    (0..count)
        .map(|index| {
            LayoutBounds::new(
                area.x,
                area.y + height * index as f32,
                area.width,
                height,
            )
        })
        .collect()
}

fn grid(
    area: LayoutBounds,
    count: usize,
    rows: usize,
    cols: usize,
) -> Vec<LayoutBounds> {
    if count == 0 {
        return Vec::new();
    }

    let cols = cols.max(1);
    let min_rows = (count + cols - 1) / cols;
    let rows = rows.max(1).max(min_rows);

    let cell_width = area.width / cols as f32;
    let cell_height = area.height / rows as f32;

    (0..count)
        .map(|index| {
            let row = index / cols;
            let col = index % cols;

            LayoutBounds::new(
                area.x + col as f32 * cell_width,
                area.y + row as f32 * cell_height,
                cell_width,
                cell_height,
            )
        })
        .collect()
}
