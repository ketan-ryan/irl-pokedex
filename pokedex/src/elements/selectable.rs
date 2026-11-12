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

#[derive(Debug, Clone)]
pub struct SectionShape {
    pub rows: usize,
    pub cols: usize,
    /// Cells per column, when columns differ in height. `None` means a full rectangle.
    col_heights: Option<Vec<usize>>,
}

impl SectionShape {
    pub fn grid(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            col_heights: None,
        }
    }

    pub fn ragged(col_heights: Vec<usize>) -> Self {
        Self {
            rows: col_heights.iter().copied().max().unwrap_or(1),
            cols: col_heights.len().max(1),
            col_heights: Some(col_heights),
        }
    }

    pub fn rows_in_col(&self, col: usize) -> usize {
        self.col_heights
            .as_ref()
            .and_then(|h| h.get(col).copied())
            .unwrap_or(self.rows)
            .max(1)
    }
}

#[derive(Debug, Clone)]
pub struct SelectionGrid {
    pub sections: Vec<SectionShape>,
    pub position: Option<SelectionPosition>,
    start_section: usize,
    start_position: Option<SelectionPosition>,
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
            start_position: None,
        }
    }

    pub fn with_start(mut self, section: usize, row: usize, col: usize) -> Self {
        self.start_position = Some(SelectionPosition { section, row, col });
        self
    }

    pub fn position(&self) -> Option<SelectionPosition> {
        self.position
    }

    fn remap_row(row: usize, from: usize, to: usize) -> usize {
        let y = (row as f32 + 0.5) / from as f32;
        ((y * to as f32) as usize).min(to - 1)
    }

    pub fn navigate(&mut self, direction: SelectionDirection) {
        let Some(current) = self.position else {
            // Nothing focused: any direction lands on the top-left region.
            self.position = if self.start_position.is_some() {
                self.start_position
            } else {
                Some(SelectionPosition {
                    section: self.start_section,
                    row: 0,
                    col: 0,
                })
            };
            return;
        };

        let shape = &self.sections[current.section];
        let SelectionPosition { section, row, col } = current;
        let mut next_position = current;

        let moved_within_section = match direction {
            SelectionDirection::Right if col + 1 < shape.cols => {
                next_position.col += 1;
                next_position.row =
                    Self::remap_row(row, shape.rows_in_col(col), shape.rows_in_col(col + 1));
                true
            }
            SelectionDirection::Left if col > 0 => {
                next_position.col -= 1;
                next_position.row =
                    Self::remap_row(row, shape.rows_in_col(col), shape.rows_in_col(col - 1));
                true
            }
            SelectionDirection::Down if row + 1 < shape.rows_in_col(col) => {
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
