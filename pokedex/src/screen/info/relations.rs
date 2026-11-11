use std::collections::{HashMap, HashSet};

use iced::{
    Alignment::Center,
    Border, Color, ContentFit, Element, Length, Padding, Point, Size, Task, Theme, alignment,
    widget::{
        Canvas, Column, Image, Row, Space, column, container, pin, responsive, scrollable, stack,
        svg, text,
    },
};
use petgraph::{
    Direction::{Incoming, Outgoing},
    algo::toposort,
    graph::{DiGraph, NodeIndex},
    visit::EdgeRef,
};

use crate::{
    elements::{
        common::{
            self, CommonAssets, action_row, holo_header_backdrop, primary_button, secondary_button,
        },
        icon_button::{IconButtonColors, IconButtonInteraction, IconButtonSize, icon_button},
        selectable::{
            SectionShape, SelectionDirection, SelectionGrid, SelectionPosition, is_focused_at,
        },
    },
    enums::{IOAction, PokemonInfo},
    screen::info::{
        arrow_layer::{ARROW, ArrowLayer},
        info::{self, Info},
    },
};

const LABEL_H: f32 = 26.0;
/// Fraction of the viewport height each scroll row occupies.
/// Below 1.0, the next row peeks out at the bottom.
const SCROLL_ROW_FRACTION: f32 = 0.8;
const ROW_SPACING: f32 = 12.0;

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
        ((x * to_cols as f32) as usize).min(to_cols - 1)
    };

    match (section, direction) {
        (section::TOP_BUTTONS, Down) => at(section::RELATIONS, 0, map_col(section::RELATIONS)),
        (section::RELATIONS, Up) => at(section::TOP_BUTTONS, 0, map_col(section::TOP_BUTTONS)),

        (section::RELATIONS, Down) => at(section::BUTTONS, 0, map_col(section::BUTTONS)),
        (section::BUTTONS, Up) => {
            let last_row = sections[section::RELATIONS].rows - 1;
            at(section::RELATIONS, last_row, map_col(section::RELATIONS))
        }

        _ => None,
    }
}

fn build_selection_grid(
    top_cols: usize,
    relations_rows: usize,
    relations_cols: usize,
) -> SelectionGrid {
    SelectionGrid::new(
        vec![
            SectionShape {
                rows: 1,
                cols: top_cols,
            },
            SectionShape {
                rows: relations_rows,
                cols: relations_cols,
            },
            SectionShape { rows: 1, cols: 7 },
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
    return_to: Option<Box<Info>>,

    evo_graph: DiGraph<String, ()>,

    arrow_icon: svg::Handle,
    close_icon: svg::Handle,

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
}

fn build_evo_graph(start: &str, get: impl Fn(&str) -> PokemonInfo) -> DiGraph<String, ()> {
    let mut g = DiGraph::new();
    let mut nodes: HashMap<String, NodeIndex> = HashMap::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut stack = vec![start.to_lowercase()];

    let idx = |g: &mut DiGraph<String, ()>, nodes: &mut HashMap<String, NodeIndex>, name: &str| {
        *nodes
            .entry(name.to_string())
            .or_insert_with(|| g.add_node(name.to_string()))
    };

    // lowercase first, then drop megas
    let neighbors = |l: Option<Vec<String>>| -> Vec<String> {
        l.unwrap_or_default()
            .into_iter()
            .map(|n| n.to_lowercase())
            .filter(|n| !n.contains("mega") && get(n).base.unwrap_or(true))
            .collect()
    };

    while let Some(name) = stack.pop() {
        if !seen.insert(name.clone()) {
            continue;
        }
        let info = get(&name);
        let me = idx(&mut g, &mut nodes, &name);

        for from in neighbors(info.evolves_from) {
            let f = idx(&mut g, &mut nodes, &from);
            g.update_edge(f, me, ());
            stack.push(from);
        }
        for into in neighbors(info.evolves_into) {
            let t = idx(&mut g, &mut nodes, &into);
            g.update_edge(me, t, ());
            stack.push(into);
        }
    }
    g
}

struct Layout {
    graph: DiGraph<String, ()>,
    selection: SelectionGrid,
}

/// Stage (column) of each node, indexed by `NodeIndex::index()`.
fn stages(g: &DiGraph<String, ()>) -> Vec<usize> {
    // If bad data creates a cycle, fall back to arbitrary order instead of panicking
    let order = toposort(g, None).unwrap_or_else(|_| g.node_indices().collect());
    let mut stage = vec![0usize; g.node_count()];
    for n in order {
        for m in g.neighbors_directed(n, Outgoing) {
            stage[m.index()] = stage[m.index()].max(stage[n.index()] + 1);
        }
    }
    stage
}

/// Nodes grouped by stage. Within a stage, order by the row of the parents
/// so branches don't cross.
fn columns(g: &DiGraph<String, ()>) -> Vec<Vec<NodeIndex>> {
    let stage = stages(g);
    let n_cols = stage.iter().max().map_or(0, |m| m + 1);
    let mut cols = vec![Vec::new(); n_cols];
    let mut row_of: HashMap<NodeIndex, usize> = HashMap::new();

    for (s, col) in cols.iter_mut().enumerate() {
        let mut nodes: Vec<NodeIndex> =
            g.node_indices().filter(|i| stage[i.index()] == s).collect();
        nodes.sort_by_key(|&n| {
            let parent_row = g
                .neighbors_directed(n, Incoming)
                .filter_map(|p| row_of.get(&p).copied())
                .min()
                .unwrap_or(0);
            (parent_row, n.index())
        });
        for (r, &n) in nodes.iter().enumerate() {
            row_of.insert(n, r);
        }
        *col = nodes;
    }
    cols
}

/// Every root-to-leaf path (used for the scrolling layout).
fn paths(g: &DiGraph<String, ()>) -> Vec<Vec<NodeIndex>> {
    fn walk(g: &DiGraph<String, ()>, path: Vec<NodeIndex>, out: &mut Vec<Vec<NodeIndex>>) {
        let last = *path.last().unwrap();
        let mut next: Vec<_> = g
            .neighbors_directed(last, Outgoing)
            .filter(|n| !path.contains(n)) // guard against cycles in bad data
            .collect();
        next.sort();
        if next.is_empty() {
            out.push(path);
            return;
        }
        for n in next {
            let mut p = path.clone();
            p.push(n);
            walk(g, p, out);
        }
    }

    let mut out = Vec::new();
    for root in g
        .node_indices()
        .filter(|&i| g.neighbors_directed(i, Incoming).next().is_none())
    {
        walk(g, vec![root], &mut out);
    }
    out
}

fn compute_layout(info: &Info) -> Layout {
    let poke_info = info.get_pokemon_info(None);

    let has_mega = poke_info
        .evolves_into
        .iter()
        .flatten()
        .any(|e| e.to_lowercase().contains("mega"));
    let top_cols = if has_mega { 2 } else { 1 };

    let name = info.get_current().key;
    let graph = build_evo_graph(&name, |n| info.get_pokemon_info(Some(n.to_string())));

    let stage = stages(&graph);
    let cols = stage.iter().max().map_or(1, |m| m + 1);

    // nodes per stage; the widest stage sets the row count
    let mut per_stage = vec![0usize; cols];
    for &s in &stage {
        per_stage[s] += 1;
    }
    let rows = per_stage.into_iter().max().unwrap_or(1).max(1);

    Layout {
        graph,
        selection: build_selection_grid(top_cols, rows, cols),
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

impl Relations {
    pub fn new(return_to: Box<Info>) -> (Self, Task<Message>) {
        let Layout { graph, selection } = compute_layout(&return_to);

        let mut this = Self {
            return_to: Some(return_to),

            evo_graph: graph,

            arrow_icon: svg::Handle::from_memory(
                include_bytes!("../../../assets/info_screen/arrow.svg").as_slice(),
            ),
            close_icon: svg::Handle::from_memory(
                include_bytes!("../../../assets/x.svg").as_slice(),
            ),

            up_interaction: IconButtonInteraction::default(),
            down_interaction: IconButtonInteraction::default(),
            close_interaction: IconButtonInteraction::default(),

            selection,
            state: RelationsState::Evolutions,
        };
        let task = this.load_graph_sprites();
        (this, task)
    }

    fn refresh(&mut self) {
        if let Some(info) = self.return_to.as_ref() {
            let Layout { graph, selection } = compute_layout(info);
            self.evo_graph = graph;
            self.selection = selection;
        }
    }

    fn step(&mut self, delta: isize) -> Action {
        let Some(info) = self.return_to.as_mut() else {
            return Action::None;
        };
        let step_task = info.step(delta).map(Message::Info);
        self.refresh();
        let sprite_task = self.load_graph_sprites();
        Action::Run(Task::batch([step_task, sprite_task]))
    }

    fn load_graph_sprites(&mut self) -> Task<Message> {
        let keys: Vec<String> = self.evo_graph.node_weights().cloned().collect();
        match self.return_to.as_mut() {
            Some(info) => info.load_sprites(&keys).map(Message::Info),
            None => Task::none(),
        }
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::IOInput(input) => match input {
                IOAction::ScrollUp => self.step(1),
                IOAction::ScrollDown => self.step(-1),
                _ => Action::None,
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
            Message::Info(msg) => match self.return_to.as_mut().unwrap().update(msg) {
                info::Action::Run(task) => Action::Run(task.map(Message::Info)),
                _ => Action::None,
            },
            Message::InfoBtn => match self.return_to.take() {
                Some(info) => Action::Return(info, Task::none()),
                None => Action::None,
            },
            Message::RelationsBtn => Action::None,
            Message::FormBtn => Action::None,
            Message::CryBtn => Action::None,
            Message::EvoBtn => {
                self.state = RelationsState::Evolutions;
                Action::None
            }
            Message::MegaBtn => {
                self.state = RelationsState::Megas;
                Action::None
            }
            Message::GigaBtn => {
                self.state = RelationsState::Gigantamax;
                Action::None
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

    fn node_cell<'a>(&'a self, name: &str, w: f32, sprite_h: f32) -> Element<'a, Message> {
        let info = self.return_to.as_ref().unwrap();

        let sprite: Element<_> = match info.get_sprite(&name.to_string()) {
            Some(h) => Image::new(h)
                .width(Length::Fill)
                .height(Length::Fixed(sprite_h))
                .content_fit(ContentFit::Contain)
                .into(),
            None => Space::new().height(sprite_h).into(),
        };

        let label = info
            .get_pokemon_info(Some(name.to_string()))
            .display_name
            .unwrap_or_else(|| capitalize(name));

        container(
            column![sprite, text(label).size(14).color(Color::WHITE)]
                .align_x(Center)
                .spacing(4),
        )
        .width(w)
        .height(sprite_h + LABEL_H)
        .into()
    }

    fn evo_view<'a>(&'a self) -> Element<'a, Message> {
        let widest = columns(&self.evo_graph)
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(1);
        if widest > 2 {
            responsive(move |size| self.evo_scroll(size)).into()
        } else {
            responsive(move |size| self.evo_fit(size)).into()
        }
    }

    /// <= 2 branches: everything fits, columns evenly spaced.
    fn evo_fit<'a>(&'a self, size: Size) -> Element<'a, Message> {
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

        // center, sprite height, and column of every node
        let mut pos: HashMap<NodeIndex, (f32, f32, f32)> = HashMap::new();
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
                    ),
                );
                col_of.insert(node, c);
            }
        }

        let arrows: Vec<(Point, f32, f32)> = g
            .edge_references()
            .filter(|e| col_of[&e.target()] == col_of[&e.source()] + 1)
            .map(|e| {
                let (ax, ay, _) = pos[&e.source()];
                let (bx, by, _) = pos[&e.target()];
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

        for (&node, &(x, y, sprite_h)) in &pos {
            let cell_h = sprite_h + LABEL_H;
            layers.push(
                pin(self.node_cell(&g[node], node_w, sprite_h))
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
    fn evo_scroll<'a>(&'a self, size: Size) -> Element<'a, Message> {
        let g = &self.evo_graph;
        let all_paths = paths(g);

        // Use the longest path so every row shares the same cell size
        let max_len = all_paths.iter().map(Vec::len).max().unwrap_or(2).max(2) as f32;

        let scrollbar_w = 12.0; // keep content clear of the scrollbar
        let usable_w = size.width - scrollbar_w;
        let node_w = (usable_w - (max_len - 1.0) * ARROW) / max_len;

        let row_h = size.height * SCROLL_ROW_FRACTION;
        let sprite_h = (row_h - LABEL_H - 8.0).max(40.0);

        let rows = all_paths.into_iter().map(|path| {
            let mut r = Row::new().align_y(Center);
            for (i, &node) in path.iter().enumerate() {
                if i > 0 {
                    r = r.push(self.arrow());
                }
                r = r.push(self.node_cell(&g[node], node_w, sprite_h));
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
        // TODO: Gigantamax
        let gigas = tab(
            "Gigantamax",
            Message::GigaBtn,
            gigas_focused,
            self.state == RelationsState::Gigantamax,
        );

        let mut top_elements = vec![evos];
        if info
            .get_pokemon_info(Some(info.get_current().key))
            .evolves_into
            .iter()
            .flatten()
            .filter(|evo| evo.contains("Mega"))
            .count()
            > 0
        {
            top_elements.push(megas);
        }

        let top_row = Row::from_vec(top_elements).spacing(10).padding(Padding {
            bottom: 15.0,
            ..Padding::ZERO
        });

        // ====================== evo graph row ======================
        let chain_background = container(self.evo_view())
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
