use super::{Structure, StructureEntry, StructureId};
use wasm_bindgen::prelude::*;

/// A structure that allow random access to structure array excluding single element.
/// It is convenient when you want to have mutable reference to two elements in the array at the same time.
pub(crate) enum StructureDynIter<'a> {
    Slice(&'a mut [StructureEntry]),
    Split {
        left_start: usize,
        left: Box<StructureDynIter<'a>>,
        right_start: usize,
        right: Box<StructureDynIter<'a>>,
        len: usize,
    },
}

impl<'a> StructureDynIter<'a> {
    pub(crate) fn new_all(source: &'a mut [StructureEntry]) -> Self {
        Self::Slice(source)
    }

    // pub(crate) fn new_offset(source: &'a mut [StructureEntry], offset: usize) -> Self {

    // }

    pub(crate) fn new(
        source: &'a mut [StructureEntry],
        split_idx: usize,
    ) -> Result<(&'a mut StructureEntry, Self), JsValue> {
        let len = source.len();
        let (left, right) = source.split_at_mut(split_idx);
        let (center, right) = right
            .split_first_mut()
            .ok_or_else(|| JsValue::from_str("Structures split fail"))?;
        Ok((
            center,
            Self::Split {
                left_start: 0,
                left: Box::new(Self::new_all(left)),
                right_start: split_idx + 1,
                right: Box::new(Self::new_all(right)),
                len,
            },
        ))
    }

    fn len(&self) -> usize {
        match self {
            Self::Slice(slice) => slice.len(),
            Self::Split { len, .. } => *len,
        }
    }

    /// Accessor without generation checking.
    #[allow(dead_code)]
    pub(crate) fn get_at(&self, idx: usize) -> Option<&StructureEntry> {
        match self {
            Self::Slice(slice) => slice.get(idx),
            Self::Split {
                left_start,
                left,
                right_start,
                right,
                ..
            } => {
                if *left_start <= idx && idx < *left_start + left.len() {
                    left.get_at(idx - *left_start)
                } else if *right_start <= idx && idx < *right_start + right.len() {
                    right.get_at(idx - *right_start)
                } else {
                    None
                }
            }
        }
    }

    /// Mutable accessor without generation checking.
    #[allow(dead_code)]
    pub(crate) fn get_at_mut<'b>(&'b mut self, idx: usize) -> Option<&'a mut StructureEntry>
    where
        'a: 'b,
    {
        let slice = match self {
            Self::Slice(slice) => Some(slice), //slice.deref_mut().first_mut(),
            _ => None,
            // Self::Split{ left_start, left, right_start, right, .. } => {
            //     if *left_start <= idx && idx < *left_start + left.len() {
            //         left.get_at_mut(idx - *left_start)
            //     } else if *right_start <= idx && idx < *right_start + right.len() {
            //         right.get_at_mut(idx - *right_start)
            //     } else {
            //         None
            //     }
            // }
        };
        if let Some(slice) = slice {
            return slice.first_mut();
        } else {
            None
        }
    }

    /// Accessor with generation checking.
    #[allow(dead_code)]
    pub(crate) fn get(&self, id: StructureId) -> Option<&dyn Structure> {
        let idx = id.id as usize;
        let entry = self.get_at(idx)?;
        if entry.gen == id.gen {
            Some(entry.dynamic?.as_ref())
        } else {
            None
        }
    }

    /// Mutable accessor with generation checking.
    pub(crate) fn get_mut<'b>(&'b mut self, id: StructureId) -> Option<&'a mut (dyn Structure)>
    where
        'a: 'b,
    {
        let idx = id.id as usize;
        let entry = self.get_at_mut(idx)?;
        if entry.gen == id.gen {
            Some(entry.dynamic?.as_mut())
        } else {
            None
        }
    }
}

pub(crate) struct StructureDynIterIter<'a, 'b> {
    src: &'a StructureDynIter<'b>,
    idx: usize,
}

impl<'a, 'b> StructureDynIterIter<'a, 'b> {
    pub(crate) fn new(src: &'a StructureDynIter<'b>) -> Self {
        Self { src, idx: 0 }
    }
}

impl<'a, 'b> Iterator for StructureDynIterIter<'a, 'b>
where
    'a: 'b,
{
    type Item = &'b dyn Structure;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ret) = self.src.get_at(self.idx) {
                if let Some(dynamic) = ret.dynamic.as_deref() {
                    return Some(dynamic);
                }
            }
            self.idx += 1;
            if self.src.len() <= self.idx {
                return None;
            }
        }
    }
}

pub(crate) struct StructureDynIterId<'a, 'b> {
    src: &'a StructureDynIter<'b>,
    idx: usize,
}

impl<'a, 'b> StructureDynIterId<'a, 'b> {
    pub(crate) fn new(src: &'a StructureDynIter<'b>) -> Self {
        Self { src, idx: 0 }
    }
}

impl<'a, 'b> Iterator for StructureDynIterId<'a, 'b>
where
    'a: 'b,
{
    type Item = (StructureId, &'b dyn Structure);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ret) = self.src.get_at(self.idx) {
                if let Some(dynamic) = ret.dynamic.as_deref() {
                    return Some((
                        StructureId {
                            id: self.idx as u32,
                            gen: ret.gen,
                        },
                        dynamic,
                    ));
                }
            }
            self.idx += 1;
            if self.src.len() <= self.idx {
                return None;
            }
        }
    }
}

pub(crate) struct StructureDynIterIterMut<'iter, 's>
where
    's: 'iter,
{
    src: &'iter mut StructureDynIter<'s>,
    idx: usize,
}

impl<'iter, 's> StructureDynIterIterMut<'iter, 's> {
    pub(crate) fn new(src: &'iter mut StructureDynIter<'s>) -> Self {
        Self { src, idx: 0 }
    }

    fn next<'t>(&'t mut self) -> Option<&'t mut dyn Structure> {
        // loop {
        if let Some(ret) = self.src.get_at_mut(self.idx) {
            if let Some(dynamic) = ret.dynamic.as_deref_mut() {
                return Some(dynamic);
            }
        }
        self.idx += 1;
        if self.src.len() <= self.idx {
            return None;
        }
        None
        // }
    }
}

impl<'iter, 's> Iterator for StructureDynIterIterMut<'iter, 's>
where
    's: 'iter,
{
    type Item = &'s mut dyn Structure;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ret) = self.src.get_at_mut(self.idx) {
                if let Some(dynamic) = ret.dynamic.as_deref_mut() {
                    return Some(dynamic);
                }
            }
            self.idx += 1;
            if self.src.len() <= self.idx {
                return None;
            }
        }
    }
}

pub(crate) struct StructureDynIterIterMutId<'a, 'b> {
    src: &'a mut StructureDynIter<'b>,
    idx: usize,
}

impl<'a, 'b> StructureDynIterIterMutId<'a, 'b> {
    pub(crate) fn new(src: &'a mut StructureDynIter<'b>) -> Self {
        Self { src, idx: 0 }
    }
}

impl<'a, 'b> Iterator for StructureDynIterIterMutId<'a, 'b>
where
    'a: 'b,
{
    type Item = (StructureId, &'b mut dyn Structure);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ret) = self.src.get_at_mut(self.idx) {
                if let Some(dynamic) = ret.dynamic.as_deref_mut() {
                    return Some((
                        StructureId {
                            id: self.idx as u32,
                            gen: ret.gen,
                        },
                        dynamic,
                    ));
                }
            }
            self.idx += 1;
            if self.src.len() <= self.idx {
                return None;
            }
        }
    }
}

// enum StructureCursor<'b> {
//     Slice(std::slice::Iter<'b, StructureEntry>),
//     Split(bool),
// }

// pub(crate) struct StructureDynIterIter<'a, 'b> {
//     src: &'a StructureDynIter<'b>,
//     cursor: Vec<StructureCursor<'a>>,
// }

// impl<'a, 'b> StructureDynIterIter<'a, 'b> {
//     pub(crate) fn new(src: &'a StructureDynIter<'b>) -> Self {
//         Self {
//             src,
//             cursor: vec![],
//         }
//     }
// }

// impl<'a, 'b> Iterator for StructureDynIterIter<'a, 'b> {
//     type Item = (StructureId, &dyn Structure);

//     fn next(&mut self) -> Option<&Self::Item> {
//         if self.cursor.is_empty() {
//             match self.src {
//                 StructureDynIter::Slice(slice) => self.cursor.push(StructureCursor::Slice(slice.iter())),
//                 StructureDynIter::Split{ left_start, left, right_start, right, len} => {
//                     self.cursor.push()
//                 }
//             }
//         }
//         if let Some(last) = self.cursor.last() {
//             match last {
//                 StructureCursor::Slice(iter) => return Some(iter.next()),
//                 StructureCursor::Split(is_right) => {
//                     let mut split = self.src;
//                     let mut is_right = self.cursor.first()?;
//                     let mut level = 0;
//                     loop {
//                         split = match split {
//                             &StructureDynIter::Split{left, right, ..} => {
//                                 if is_right {
//                                     right
//                                 } else {
//                                     left
//                                 }
//                             }
//                         };
//                         level += 1;
//                         is_right = self.cursor[level]
//                         if is_right {
//                             self.cursor.push(self.src.)
//                 }
//             }
//         }
//     }
