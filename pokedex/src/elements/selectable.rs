#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionPosition {
    pub section: usize,
    pub row: usize,
    pub col: usize,
}
#[derive(Debug, Clone, Copy)]
pub struct SectionShape {
    pub rows: usize,
    pub cols: usize,
}

#[derive(Debug, Clone)]
pub struct SelectionGrid {
    pub sections: Vec<SectionShape>,
    pub position: Option<SelectionPosition>,
}
