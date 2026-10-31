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
    start_section: usize,
    escape_fn: fn(
        sections: &[SectionShape],
        section: usize,
        row: usize,
        col: usize,
        direction: SelectionDirection,
    ) -> Option<SelectionPosition>,
}

impl SelectionGrid {
    pub fn new(
        sections: Vec<SectionShape>,
        start_section: usize,
        escape_fn: fn(
            sections: &[SectionShape],
            section: usize,
            row: usize,
            col: usize,
            direction: SelectionDirection,
        ) -> Option<SelectionPosition>,
    ) -> Self {
        Self {
            sections,
            position: None,
            start_section,
            escape_fn,
        }
    }

    pub fn position(&self) -> Option<SelectionPosition> {
        self.position
    }

    pub fn navigate(&mut self, direction: SelectionDirection) {
        let Some(current) = self.position else {
            // Nothing focused: any direction lands on the top-left region.
            self.position = Some(SelectionPosition {
                section: self.start_section,
                row: 0,
                col: 0,
            });
            return;
        };

        let shape = self.sections[current.section];
        let SelectionPosition { section, row, col } = current;
        let mut next_position = current;

        let moved_within_section = match direction {
            SelectionDirection::Right if col + 1 < shape.cols => {
                next_position.col += 1;
                true
            }
            SelectionDirection::Left if col > 0 => {
                next_position.col -= 1;
                true
            }
            SelectionDirection::Down if row + 1 < shape.rows => {
                next_position.row += 1;
                true
            }
            SelectionDirection::Up if row > 0 => {
                next_position.row -= 1;
                true
            }
            _ => false,
        };

        self.position = if moved_within_section {
            Some(next_position)
        } else {
            // escape() returning None means "no neighbor that way" -- i.e.
            // off the edge of the page, so focus clears entirely rather
            // than clamping in place.
            (self.escape_fn)(&self.sections, section, row, col, direction)
        };
    }
}

pub fn is_focused_at(
    focus: Option<SelectionPosition>,
    section: usize,
    row: usize,
    col: usize,
) -> bool {
    focus.is_some_and(|f| f.section == section && f.row == row && f.col == col)
}
