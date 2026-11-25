use std::collections::HashMap;

use iced::{
    Alignment::Center,
    Border, Color, ContentFit, Element, Length, Padding, Point, Size, Task, Theme, alignment,
    widget::{
        Canvas, Column, Image, Row, Space, column, container, pin, responsive, scrollable, stack,
        svg, text,
    },
};
use log::{debug, warn};
use petgraph::{
    graph::{DiGraph, NodeIndex},
    visit::EdgeRef,
};

use crate::{
    elements::{
        common::{
            self, CommonAssets, action_row, holo_header_backdrop, primary_button, secondary_button,
        },
        evo_graph::{build_evo_graph, columns, locate, node_at, paths, stages},
        icon_button::{IconButtonColors, IconButtonInteraction, IconButtonSize, icon_button},
        selectable::{
            SectionShape, SelectionDirection, SelectionGrid, SelectionPosition, is_focused_at,
        },
    },
    enums::IOAction,
    screen::info::{
        arrow_layer::{ARROW, ArrowLayer},
        info::{self, Info},
    },
};

pub const LABEL_H: f32 = 26.0;
/// Fraction of the viewport height each scroll row occupies.
/// Below 1.0, the next row peeks out at the bottom.
pub const SCROLL_ROW_FRACTION: f32 = 0.8;
pub const ROW_SPACING: f32 = 12.0;

mod section {
    pub const TOP_BUTTONS: usize = 0;
    pub const RELATIONS: usize = 1;
    pub const BUTTONS: usize = 2;
}

fn escape(
    sections: &[SectionShape],
    section: usize,
    _row: usize,
    col: usize,
    direction: SelectionDirection,
) -> Option<SelectionPosition> {
    use SelectionDirection::*;

    let at = |section: usize, row: usize, col: usize| Some(SelectionPosition { section, row, col });

    // Column in `to` that sits under the current column's horizontal center.
    let map_col = |to: usize| {
        let from_cols = sections[section].cols as f32;
        let to_cols = sections[to].cols;
        let x = (col as f32 + 0.5) / from_cols;
        ((x * to_cols as f32) as usize).min(to_cols.saturating_sub(1))
    };

    match (section, direction) {
        (section::TOP_BUTTONS, Down) => at(section::RELATIONS, 0, map_col(section::RELATIONS)),
        (section::RELATIONS, Up) => at(section::TOP_BUTTONS, 0, map_col(section::TOP_BUTTONS)),

        (section::RELATIONS, Down) => at(section::BUTTONS, 0, map_col(section::BUTTONS)),
        (section::BUTTONS, Up) => {
            let c = map_col(section::RELATIONS);
            at(
                section::RELATIONS,
                sections[section::RELATIONS].rows_in_col(c) - 1,
                c,
            )
        }

        _ => None,
    }
}

fn build_selection_grid(top_cols: usize, relations: SectionShape) -> SelectionGrid {
    SelectionGrid::new(
        vec![
            SectionShape::grid(1, top_cols),
            relations,
            SectionShape::grid(1, 7),
        ],
        section::RELATIONS,
        escape,
    )
}

#[derive(Clone, Debug, PartialEq)]
enum RelationsState {
    Evolutions,
    Megas,
    Gigantamax,
}

#[derive(Debug)]
pub struct Relations {
    pub return_to: Option<Box<Info>>,

    evo_graph: DiGraph<String, ()>,

    arrow_icon: svg::Handle,
    close_icon: svg::Handle,
    outline_svg: svg::Handle,

    up_interaction: IconButtonInteraction,
    down_interaction: IconButtonInteraction,
    close_interaction: IconButtonInteraction,

    selection: SelectionGrid,
    state: RelationsState,
}

#[derive(Clone, Debug)]
pub enum Message {
    IOInput(IOAction),

    UpInteraction(IconButtonInteraction),
    DownInteraction(IconButtonInteraction),
    CloseInteraction(IconButtonInteraction),

    EvoBtn,
    MegaBtn,
    GigaBtn,

    JumpToPokemon(String),

    InfoBtn,
    RelationsBtn,
    FormBtn,
    CryBtn,

    Info(info::Message),
}

pub enum Action {
    None,
    Return(Box<Info>, Task<crate::info::Message>),
    Run(Task<Message>),
    OpenForms,
}

struct Layout {
    graph: DiGraph<String, ()>,
    selection: SelectionGrid,
}

fn compute_layout(info: &Info, state: &RelationsState) -> Layout {
    let poke_info = info.get_pokemon_info(None);

    let has_mega = poke_info.mega.as_ref().is_some_and(|megas| megas.len() > 0);
    let has_gmax = poke_info.gmax.is_some();

    let top_cols = if has_mega && has_gmax {
        3
    } else if has_mega || has_gmax {
        2
    } else {
        1
    };

    let name = info.get_current().key;
    let graph = build_evo_graph(&name, |n| info.get_pokemon_info(Some(n.to_string())));
    let stage = stages(&graph);
    let cols = stage.iter().max().map_or(1, |m| m + 1);

    // nodes per stage; the widest stage sets the row count
    let mut per_stage = vec![0usize; cols];
    for &s in &stage {
        per_stage[s] += 1;
    }

    // find widest stage
    let cols = columns(&graph);
    let widest = cols.iter().map(Vec::len).max().unwrap_or(1);

    let scroll_mode = widest > 2;
    let current = locate(&graph, &cols, scroll_mode, &name.to_lowercase());

    let relations: SectionShape = match state {
        RelationsState::Evolutions => {
            if widest > 2 {
                // scroll mode: one row per path, so every row is a full rectangle
                let ps = paths(&graph);
                let max_len = ps.iter().map(Vec::len).max().unwrap_or(1);
                SectionShape::grid(ps.len(), max_len)
            } else {
                SectionShape::ragged(cols.iter().map(Vec::len).collect())
            }
        }
        RelationsState::Megas => SectionShape::grid(1, poke_info.mega.unwrap_or_default().len()),
        RelationsState::Gigantamax => SectionShape::grid(1, 1),
    };

    debug!("Section shape {:?} with state {:?}", relations, state);

    let mut selection = build_selection_grid(top_cols, relations);
    if *state == RelationsState::Evolutions
        && let Some((row, col)) = current
    {
        selection = selection.with_start(section::RELATIONS, row, col);
        debug!("Starting at position {} {}", row, col);
    }

    Layout {
        graph,
        selection: selection,
    }
}

pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

impl Relations {
    pub fn new(return_to: Box<Info>) -> (Self, Task<Message>) {
        let start_state = RelationsState::Evolutions;
        let Layout { graph, selection } = compute_layout(&return_to, &start_state);

        let mut this = Self {
            return_to: Some(return_to),

            evo_graph: graph,

            arrow_icon: svg::Handle::from_memory(
                include_bytes!("../../../assets/info_screen/arrow.svg").as_slice(),
            ),
            close_icon: svg::Handle::from_memory(
                include_bytes!("../../../assets/x.svg").as_slice(),
            ),
            outline_svg: svg::Handle::from_memory(
                include_bytes!("../../../assets/info_screen/sprite_outline.svg").as_slice(),
            ),

            up_interaction: IconButtonInteraction::default(),
            down_interaction: IconButtonInteraction::default(),
            close_interaction: IconButtonInteraction::default(),

            selection,
            state: start_state,
        };
        let task = this.load_graph_sprites();
        (this, task)
    }

    fn refresh(&mut self) {
        if let Some(info) = self.return_to.as_ref() {
            let Layout { graph, selection } = compute_layout(info, &self.state);
            self.evo_graph = graph;
            self.selection = selection;

            self.close_interaction = IconButtonInteraction::None;
            self.down_interaction = IconButtonInteraction::None;
            self.up_interaction = IconButtonInteraction::None;
        }
    }

    fn step(&mut self, delta: isize) -> Action {
        let Some(info) = self.return_to.as_mut() else {
            warn!("No info to return to from step!");
            return Action::None;
        };
        let step_task = info.step(delta).map(Message::Info);
        self.state = RelationsState::Evolutions;
        self.refresh();
        let sprite_task = self.load_graph_sprites();
        Action::Run(Task::batch([step_task, sprite_task]))
    }

    fn load_graph_sprites(&mut self) -> Task<Message> {
        let state = self.state.clone();
        match self.return_to.as_mut() {
            Some(info) => {
                let current = info.get_pokemon_info(Some(info.get_current().key));
                let mut evos: Vec<String> = self.evo_graph.node_weights().cloned().collect();
                evos.extend(current.mega.unwrap_or_default());
                evos.extend(current.gmax.into_iter());
                debug!(
                    "Loading sprites for {:?} with current {:?} and state {:?} with full list {:?}",
                    current.display_name,
                    info.get_current(),
                    state,
                    evos
                );
                info.load_sprites(&evos).map(Message::Info)
            }
            None => Task::none(),
        }
    }

    fn focused_message(&self) -> Option<Message> {
        let SelectionPosition { section, row, col } = self.selection.position?;

        match section {
            section::TOP_BUTTONS => match col {
                0 => Some(Message::EvoBtn),
                1 => Some(Message::MegaBtn),
                2 => Some(Message::GigaBtn),
                _ => None,
            },
            section::RELATIONS => {
                let cols = columns(&self.evo_graph);
                let scroll_mode = cols.iter().map(Vec::len).max().unwrap_or(1) > 2;

                let info_ref = self.return_to.as_ref().unwrap();
                let curr_info = info_ref.get_pokemon_info(Some(info_ref.get_current().key));

                let key = match self.state {
                    RelationsState::Evolutions => {
                        node_at(&self.evo_graph, &cols, scroll_mode, row, col)
                            .map(|n| self.evo_graph[n].clone())
                    }
                    RelationsState::Megas => {
                        curr_info.mega.as_ref().and_then(|m| m.get(col)).cloned()
                    }
                    RelationsState::Gigantamax => curr_info.gmax.clone(),
                };
                let Some(key) = key else {
                    return None;
                };

                Some(Message::JumpToPokemon(key))
            }
            section::BUTTONS => match col {
                0 => Some(Message::UpInteraction(IconButtonInteraction::Released)),
                1 => Some(Message::DownInteraction(IconButtonInteraction::Released)),
                2 => Some(Message::InfoBtn),
                3 => Some(Message::RelationsBtn),
                4 => Some(Message::FormBtn),
                5 => Some(Message::CryBtn),
                6 => Some(Message::CloseInteraction(IconButtonInteraction::Released)),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::IOInput(input) => match input {
                IOAction::ScrollUp => {
                    if self.selection.position.is_none() {
                        Action::Run(Task::done(Message::UpInteraction(
                            IconButtonInteraction::Released,
                        )))
                    } else {
                        self.selection.navigate(SelectionDirection::Up);
                        Action::None
                    }
                }
                IOAction::ScrollDown => {
                    if self.selection.position.is_none() {
                        Action::Run(Task::done(Message::DownInteraction(
                            IconButtonInteraction::Released,
                        )))
                    } else {
                        self.selection.navigate(SelectionDirection::Down);
                        Action::None
                    }
                }
                IOAction::Left => {
                    self.selection.navigate(SelectionDirection::Left);
                    Action::None
                }
                IOAction::Right => {
                    self.selection.navigate(SelectionDirection::Right);
                    Action::None
                }
                IOAction::Select => match self.focused_message() {
                    Some(message) => self.update(message),
                    None => Action::None,
                },
            },
            Message::UpInteraction(i) => {
                if i == IconButtonInteraction::Released {
                    self.up_interaction = IconButtonInteraction::Hovered;
                    return self.step(1);
                }
                self.up_interaction = i;
                Action::None
            }
            Message::DownInteraction(i) => {
                if i == IconButtonInteraction::Released {
                    self.down_interaction = IconButtonInteraction::Hovered;
                    return self.step(-1);
                }
                self.down_interaction = i;
                Action::None
            }
            Message::CloseInteraction(i) => {
                if i == IconButtonInteraction::Released {
                    self.close_interaction = IconButtonInteraction::Hovered;
                    match self.return_to.take() {
                        Some(info) => Action::Return(info, Task::none()),
                        None => Action::None,
                    }
                } else {
                    self.close_interaction = i;
                    Action::None
                }
            }
            Message::JumpToPokemon(key) => {
                let Some(info) = self.return_to.as_mut() else {
                    return Action::None;
                };
                if info.get_current().key.eq_ignore_ascii_case(&key) {
                    return Action::None; // already showing it
                }

                debug!("Jumping to {}", key);
                let jump = info.jump_to(&key);
                self.state = RelationsState::Evolutions;
                self.refresh();
                let sprites = self.load_graph_sprites();
                Action::Run(Task::batch([jump.map(Message::Info), sprites]))
            }
            Message::Info(msg) => match self.return_to.as_mut().unwrap().update(msg) {
                info::Action::Run(task) => Action::Run(task.map(Message::Info)),
                _ => Action::None,
            },
            Message::InfoBtn => match self.return_to.take() {
                Some(info) => Action::Return(info, Task::none()),
                None => Action::None,
            },
            Message::RelationsBtn => Action::None,
            Message::FormBtn => Action::OpenForms,
            Message::CryBtn => Action::None,
            Message::EvoBtn => {
                self.state = RelationsState::Evolutions;
                Action::None
            }
            Message::MegaBtn => {
                self.state = RelationsState::Megas;
                debug!("Mega pressed");
                self.refresh();
                Action::Run(self.load_graph_sprites())
            }
            Message::GigaBtn => {
                self.state = RelationsState::Gigantamax;
                debug!("Giga pressed");
                self.refresh();
                Action::Run(self.load_graph_sprites())
            }
        }
    }

    /// `angle` is the direction the arrow points, in radians (0 = right, y down).
    fn arrow<'a>(&self) -> Element<'a, Message> {
        Canvas::new(ArrowLayer {
            arrows: vec![(Point::new(ARROW / 2.0, ARROW / 2.0), 0.0, ARROW)],
        })
        .width(ARROW)
        .height(ARROW)
        .into()
    }

    fn node_cell<'a>(
        &'a self,
        name: &str,
        w: f32,
        sprite_h: f32,
        focused: bool,
    ) -> Element<'a, Message> {
        let info = self.return_to.as_ref().unwrap();
        let unfocused = self.selection.position.is_none();

        let outline_color = if unfocused {
            Color::from_rgb8(180, 180, 180)
        } else {
            common::colors::SECONDARY_FOCUS
        };

        let img_h = sprite_h.min(w);

        let buff_w = w * 0.10;
        let buff_h = img_h * 0.10;

        let outline = svg(self.outline_svg.clone())
            .width(Length::Fixed(w + buff_w))
            .height(Length::Fixed(img_h + buff_h))
            .style(move |_, _| svg::Style {
                color: Some(outline_color),
            });

        let sprite: Element<_> = match info.get_sprite(&name.to_string()) {
            Some(h) => Image::new(h)
                .width(Length::Fill)
                .height(Length::Fixed(img_h))
                .content_fit(ContentFit::Contain)
                .into(),
            None => Space::new().height(img_h).into(),
        };

        let current = info.get_current().key;
        let should_outline = match self.state {
            RelationsState::Evolutions => name == current,
            RelationsState::Megas => info
                .get_pokemon_info(Some(current))
                .mega
                .is_some_and(|megas| megas.get(0) == Some(&name.to_string())),
            RelationsState::Gigantamax => info
                .get_pokemon_info(Some(current))
                .gmax
                .is_some_and(|gmax| gmax == name),
        };

        let content: Element<_> = if (should_outline && unfocused) || focused {
            stack![sprite, outline].into()
        } else {
            sprite.into()
        };

        let label = info
            .get_pokemon_info(Some(name.to_string()))
            .display_name
            .unwrap_or_else(|| capitalize(name));

        container(
            column![
                content,
                text(label).size(14).align_x(Center).color(Color::WHITE)
            ]
            .align_x(Center)
            .spacing(4),
        )
        .width(w)
        .center_y(sprite_h + LABEL_H)
        .into()
    }

    /// Draws `names` as evenly spaced node cells with no arrows.
    /// Sizing matches `evo_fit` for a single row of columns.
    fn node_row<'a>(
        &'a self,
        names: &[String],
        size: Size,
        focus: &Option<SelectionPosition>,
        section_id: usize,
    ) -> Element<'a, Message> {
        let n = names.len().max(1) as f32;
        let col_w = size.width / n;

        // same gutter rule as evo_fit, so sprites are the same width as in a 2-stage layout
        let gutter = if n > 1.0 {
            (col_w * 0.30).max(ARROW + 16.0)
        } else {
            0.0
        };
        let node_w = col_w - gutter;
        let sprite_h = (size.height - LABEL_H - 8.0).max(40.0);
        let cell_h = sprite_h + LABEL_H;

        let cells = names.iter().enumerate().map(|(col, name)| {
            let focused = is_focused_at(*focus, section_id, 0, col);
            pin(self.node_cell(name, node_w, sprite_h, focused))
                .x((col as f32 + 0.5) * col_w - node_w / 2.0)
                .y((size.height - cell_h) / 2.0)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        });

        stack(cells.collect::<Vec<Element<'a, Message>>>())
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn evo_view<'a>(&'a self) -> Element<'a, Message> {
        let focus = self.selection.position();
        let widest = columns(&self.evo_graph)
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(1);
        if widest > 2 {
            responsive(move |size| self.evo_scroll(size, &focus)).into()
        } else {
            responsive(move |size| self.evo_fit(size, &focus)).into()
        }
    }

    /// <= 2 branches: everything fits, columns evenly spaced.
    fn evo_fit<'a>(
        &'a self,
        size: Size,
        focus: &Option<SelectionPosition>,
    ) -> Element<'a, Message> {
        let g = &self.evo_graph;
        let cols = columns(g);
        let n_cols = cols.len().max(1) as f32;

        let col_w = size.width / n_cols;
        // Gutter for arrows scales with column width; 3-stage layouts get more room
        let gutter = if n_cols > 1.0 {
            (col_w * 0.30).max(ARROW + 16.0)
        } else {
            0.0
        };
        let node_w = col_w - gutter;
        let arrow_len = (gutter * 0.75).clamp(ARROW, 64.0);

        // x, y, sprite_h, row, col
        let mut pos: HashMap<NodeIndex, (f32, f32, f32, usize, usize)> = HashMap::new();
        let mut col_of: HashMap<NodeIndex, usize> = HashMap::new();
        for (c, col) in cols.iter().enumerate() {
            let n = col.len() as f32;
            // size by this column's own node count, so unsplit columns grow
            let sprite_h = (size.height / n - LABEL_H - 8.0).max(40.0);
            for (r, &node) in col.iter().enumerate() {
                pos.insert(
                    node,
                    (
                        (c as f32 + 0.5) * col_w,
                        size.height * (r as f32 + 0.5) / n,
                        sprite_h,
                        r,
                        c,
                    ),
                );
                col_of.insert(node, c);
            }
        }

        let arrows: Vec<(Point, f32, f32)> = g
            .edge_references()
            .filter(|e| col_of[&e.target()] == col_of[&e.source()] + 1)
            .map(|e| {
                let (ax, ay, _, _, _) = pos[&e.source()];
                let (bx, by, _, _, _) = pos[&e.target()];
                (
                    Point::new((ax + bx) / 2.0, (ay + by) / 2.0),
                    (by - ay).atan2(bx - ax),
                    arrow_len,
                )
            })
            .collect();

        let mut layers: Vec<Element<'a, Message>> = vec![
            Canvas::new(ArrowLayer { arrows })
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        ];

        for (&node, &(x, y, sprite_h, row, col)) in &pos {
            let cell_h = sprite_h + LABEL_H;
            let focused = is_focused_at(*focus, section::RELATIONS, row, col);
            layers.push(
                pin(self.node_cell(&g[node], node_w, sprite_h, focused))
                    .x(x - node_w / 2.0)
                    .y(y - cell_h / 2.0)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into(),
            );
        }

        stack(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// > 2 branches: one scrollable row per evolution path, each sized like a
    /// plain 2-stage layout but slightly shorter than the viewport.
    fn evo_scroll<'a>(
        &'a self,
        size: Size,
        focus: &Option<SelectionPosition>,
    ) -> Element<'a, Message> {
        let g = &self.evo_graph;
        let all_paths = paths(g);

        // Use the longest path so every row shares the same cell size
        let max_len = all_paths.iter().map(Vec::len).max().unwrap_or(2).max(2) as f32;

        let scrollbar_w = 12.0; // keep content clear of the scrollbar
        let usable_w = size.width - scrollbar_w;
        let node_w = (usable_w - (max_len - 1.0) * ARROW) / max_len;

        let row_h = size.height * SCROLL_ROW_FRACTION;
        let sprite_h = (row_h - LABEL_H - 8.0).max(40.0);

        let rows = all_paths.into_iter().enumerate().map(|(row, path)| {
            let mut r = Row::new().align_y(Center);
            for (col, &node) in path.iter().enumerate() {
                if col > 0 {
                    r = r.push(self.arrow());
                }
                let focused = is_focused_at(*focus, section::RELATIONS, row, col);
                r = r.push(self.node_cell(&g[node], node_w, sprite_h, focused));
            }
            container(r)
                .width(Length::Fill)
                .height(Length::Fixed(row_h))
                .center_x(Length::Fill)
                .center_y(Length::Fixed(row_h))
                .into()
        });

        scrollable(
            Column::with_children(rows)
                .spacing(ROW_SPACING)
                .width(Length::Fill),
        )
        .style(|_theme, status| {
            let scroller_color = match status {
                scrollable::Status::Dragged { .. } => Color::from_rgba8(15, 0, 150, 1.0),
                scrollable::Status::Hovered { .. } => Color::from_rgba8(15, 0, 150, 0.9),
                _ => Color::from_rgba8(15, 0, 150, 0.6),
            };
            scrollable::Style {
                container: Default::default(),
                vertical_rail: scrollable::Rail {
                    background: Some(iced::Background::Color(Color::from_rgba8(0, 0, 0, 0.1))),
                    border: Border {
                        radius: 4.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                    scroller: scrollable::Scroller {
                        background: iced::Background::Color(scroller_color),
                        border: Border {
                            radius: 4.0.into(),
                            width: 0.0,
                            color: Color::TRANSPARENT,
                        },
                    },
                },
                horizontal_rail: scrollable::Rail {
                    background: None,
                    border: Border::default(),
                    scroller: scrollable::Scroller {
                        background: iced::Background::Color(Color::TRANSPARENT),
                        border: Border::default(),
                    },
                },
                gap: None,
                auto_scroll: scrollable::AutoScroll {
                    background: iced::Background::Color(Color::TRANSPARENT),
                    border: Border::default(),
                    shadow: Default::default(),
                    icon: Color::TRANSPARENT,
                },
            }
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    pub fn top_view<'a>(&'a self, common: &'a CommonAssets) -> Element<'a, Message> {
        let bg = holo_header_backdrop(common, "Pokémon Info", "Relations view");
        let info = self.return_to.as_ref().unwrap();
        let image = info.get_sprite(&info.get_current().key);
        let sprite: Element<_> = match image {
            Some(h) => container(Image::new(h).width(Length::Fill))
                .width(Length::Fixed(320.0))
                .into(),
            None => Space::new().into(),
        };

        stack![
            bg,
            container(sprite)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Bottom)
                .padding(8)
        ]
        .into()
    }

    pub fn bottom_view<'a>(&'a self, common: &'a CommonAssets) -> Element<'a, Message> {
        let focus: Option<SelectionPosition> = self.selection.position();
        let info = self.return_to.as_ref().unwrap();

        // ====================== top button row ======================
        let evos_focused = is_focused_at(focus, section::TOP_BUTTONS, 0, 0);
        let megas_focused = is_focused_at(focus, section::TOP_BUTTONS, 0, 1);
        let gigas_focused = is_focused_at(focus, section::TOP_BUTTONS, 0, 2);

        let tab = |label: &str, msg: Message, focused, active: bool| {
            if active {
                primary_button(label.to_string(), msg, focused)
            } else {
                secondary_button(label.to_string(), msg, None, focused, false)
            }
        };

        let evos = tab(
            "Evolution",
            Message::EvoBtn,
            evos_focused,
            self.state == RelationsState::Evolutions,
        );
        let megas = tab(
            "Mega Evolution",
            Message::MegaBtn,
            megas_focused,
            self.state == RelationsState::Megas,
        );
        let gigas = tab(
            "Gigantamax",
            Message::GigaBtn,
            gigas_focused,
            self.state == RelationsState::Gigantamax,
        );

        let mut top_elements = vec![evos];
        if info
            .get_pokemon_info(Some(info.get_current().key))
            .mega
            .is_some_and(|megas| megas.len() > 0)
        {
            top_elements.push(megas);
        }

        if info
            .get_pokemon_info(Some(info.get_current().key))
            .gmax
            .is_some()
        {
            top_elements.push(gigas);
        }

        let top_row = Row::from_vec(top_elements).spacing(10).padding(Padding {
            bottom: 15.0,
            ..Padding::ZERO
        });

        let center: Element<_> = match self.state {
            RelationsState::Evolutions => self.evo_view(),
            RelationsState::Megas => {
                let megas: Vec<String> = info
                    .get_pokemon_info(Some(info.get_current().key))
                    .mega
                    .iter()
                    .flatten()
                    .cloned()
                    .collect();
                responsive(move |size| self.node_row(&megas, size, &focus, section::RELATIONS))
                    .into()
            }
            RelationsState::Gigantamax => {
                match info.get_pokemon_info(Some(info.get_current().key)).gmax {
                    Some(gmax) => responsive(move |size| {
                        self.node_row(&[gmax.clone()], size, &focus, section::RELATIONS)
                    })
                    .into(),
                    None => text(format!(
                        "Failed to retrieve gmax form for {}",
                        info.get_current().key
                    ))
                    .into(),
                }
            }
        };

        // ====================== evo graph row ======================
        let chain_background = container(center)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .style(|_: &Theme| container::Style {
                border: iced::Border {
                    color: Color::WHITE,
                    width: 2.0,
                    radius: 20.0.into(),
                },
                background: Some(iced::Background::Color(common::colors::CARD_BORDER)),
                ..Default::default()
            });

        // ====================== bottom button row ======================
        let up_focused = is_focused_at(focus, section::BUTTONS, 0, 0);
        let down_focused = is_focused_at(focus, section::BUTTONS, 0, 1);
        let info_focused = is_focused_at(focus, section::BUTTONS, 0, 2);
        let relations_focused = is_focused_at(focus, section::BUTTONS, 0, 3);
        let forms_focused = is_focused_at(focus, section::BUTTONS, 0, 4);
        let cry_focused = is_focused_at(focus, section::BUTTONS, 0, 5);
        let close_focused = is_focused_at(focus, section::BUTTONS, 0, 6);
        let size = IconButtonSize::large();
        let colors = IconButtonColors {
            default_shadow: common::default_shadow(),
            ..Default::default()
        };
        let up = icon_button(
            self.arrow_icon.clone(),
            IconButtonSize {
                rotation: Some(iced::Rotation::Floating(iced::Radians::PI)),
                ..IconButtonSize::large()
            },
            None,
            &self.up_interaction,
            colors,
            Message::UpInteraction,
            up_focused,
        );
        let down = icon_button(
            self.arrow_icon.clone(),
            size,
            None,
            &self.down_interaction,
            colors,
            Message::DownInteraction,
            down_focused,
        );
        let close = icon_button(
            self.close_icon.clone(),
            size,
            None,
            &self.close_interaction,
            colors,
            Message::CloseInteraction,
            close_focused,
        );
        let info_button = secondary_button(
            "Info".to_string(),
            Message::InfoBtn,
            None,
            info_focused,
            false,
        );
        let relations_button = primary_button(
            "Relations".to_string(),
            Message::RelationsBtn,
            relations_focused,
        );
        let forms_button = secondary_button(
            "Forms".to_string(),
            Message::FormBtn,
            None,
            forms_focused,
            false,
        );
        let cry_button =
            secondary_button("Cry".to_string(), Message::CryBtn, None, cry_focused, false);
        let elements = vec![
            up,
            down,
            info_button,
            relations_button,
            forms_button,
            cry_button,
            close.into(),
        ];
        let row = action_row(elements);
        stack![
            container(
                Canvas::new(&common.scanlines)
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(common::colors::BACKGROUND)),
                ..Default::default()
            }),
            column![top_row, chain_background, row].padding(8)
        ]
        .into()
    }
}
