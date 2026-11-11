use std::collections::{HashMap, HashSet};

use iced::{
    Element, Length, Task, alignment,
    widget::{Canvas, Image, Space, container, stack, svg},
};
use petgraph::{
    Direction::Outgoing,
    algo::toposort,
    graph::{DiGraph, NodeIndex},
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
    screen::info::info::{self, Info},
};

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
}

#[derive(Clone, Debug)]
pub enum Message {
    IOInput(IOAction),

    UpInteraction(IconButtonInteraction),
    DownInteraction(IconButtonInteraction),
    CloseInteraction(IconButtonInteraction),

    InfoBtn,
    Relations,
    Form,
    Cry,

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
            .filter(|n| !n.contains("mega"))
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

impl Relations {
    pub fn new(return_to: Box<Info>) -> (Self, Task<Message>) {
        let Layout { graph, selection } = compute_layout(&return_to);

        (
            Self {
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
            },
            Task::none(),
        )
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
        let task = info.step(delta);
        self.refresh();
        Action::Run(task.map(Message::Info))
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
            Message::InfoBtn => Action::None,
            Message::Relations => Action::None,
            Message::Form => Action::None,
            Message::Cry => Action::None,
        }
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
        let info_button = primary_button("Info".to_string(), Message::InfoBtn, info_focused);
        let relations_button = secondary_button(
            "Relations".to_string(),
            Message::Relations,
            None,
            relations_focused,
            false,
        );
        let forms_button = secondary_button(
            "Forms".to_string(),
            Message::Form,
            None,
            forms_focused,
            false,
        );
        let cry_button =
            secondary_button("Cry".to_string(), Message::Cry, None, cry_focused, false);
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
            container(row)
                .width(Length::Fill)
                .align_bottom(Length::Fill)
                .padding(8)
        ]
        .into()
    }
}
