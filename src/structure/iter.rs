use super::{Structure, StructureEntry, StructureId};
use crate::dyn_iter::{DynIter, DynIterMut};
use wasm_bindgen::prelude::*;

#[derive(Default)]
pub(crate) struct StructureSlice<'a> {
    start: usize,
    slice: &'a mut [StructureEntry],
}

/// A structure that allow random access to structure array with possible gaps.
/// It uses a Vec of slices, which will use dynamic memory, which is a bit sad, but we can allow any number
/// of slices and access internal object in O(n) where n is the number of slices, not the number of objects.
/// It is convenient when you want to have mutable reference to two elements in the array at the same time.
pub(crate) struct StructureDynIter<'a>(Vec<StructureSlice<'a>>);

impl<'a> StructureDynIter<'a> {
    pub(crate) fn new_all(source: &'a mut [StructureEntry]) -> Self {
        Self(vec![StructureSlice {
            start: 0,
            slice: source,
        }])
    }

    pub(crate) fn new(
        source: &'a mut [StructureEntry],
        split_idx: usize,
    ) -> Result<(&'a mut StructureEntry, Self), JsValue> {
        let (left, right) = source.split_at_mut(split_idx);
        let (center, right) = right
            .split_first_mut()
            .ok_or_else(|| JsValue::from_str("Structures split fail"))?;
        Ok((
            center,
            Self(vec![
                StructureSlice {
                    start: 0,
                    slice: left,
                },
                StructureSlice {
                    start: split_idx + 1,
                    slice: right,
                },
            ]),
        ))
    }

    #[allow(dead_code)]
    pub(crate) fn exclude(&mut self, idx: usize) -> Result<&mut StructureEntry, JsValue> {
        if let Some((slice_idx, _)) = self
            .0
            .iter_mut()
            .enumerate()
            .find(|(_, slice)| slice.start <= idx && idx < slice.start + slice.slice.len())
        {
            let slice = std::mem::take(&mut self.0[slice_idx]);
            let (left, right) = slice.slice.split_at_mut(idx - slice.start);
            let (center, right) = right
                .split_first_mut()
                .ok_or_else(|| js_str!("Structure split fail"))?;
            self.0[slice_idx] = StructureSlice {
                start: slice.start,
                slice: left,
            };
            self.0.push(StructureSlice {
                start: idx,
                slice: right,
            });
            Ok(center)
        } else {
            js_err!("Strucutre slices out of range")
        }
    }

    pub(crate) fn exclude_id<'b>(
        &'b mut self,
        id: StructureId,
    ) -> Result<
        (
            Option<&'b mut (dyn Structure + 'static)>,
            StructureDynIter<'b>,
        ),
        JsValue,
    >
    where
        'a: 'b,
    {
        let idx = id.id as usize;
        if let Some((slice_idx, _)) = self
            .0
            .iter()
            .enumerate()
            .find(|(_, slice)| slice.start <= idx && idx < slice.start + slice.slice.len())
        {
            let slice_borrow = &self.0[slice_idx];
            let entry = &slice_borrow.slice[idx - slice_borrow.start];
            if entry.gen != id.gen || entry.dynamic.is_none() {
                return Ok((
                    None,
                    StructureDynIter(
                        self.0
                            .iter_mut()
                            .map(|i| StructureSlice {
                                start: i.start,
                                slice: &mut i.slice[..],
                            })
                            .collect(),
                    ),
                ));
            }
            let (left_slices, right_slices) = self.0.split_at_mut(slice_idx);
            let (slice, right_slices) = right_slices
                .split_first_mut()
                .ok_or_else(|| js_str!("Structure slice split fail"))?;

            let (left, right) = slice.slice.split_at_mut(idx - slice.start);
            let (center, right) = right
                .split_first_mut()
                .ok_or_else(|| js_str!("Structure split fail"))?;

            let left_slices = left_slices
                .iter_mut()
                .map(|i| StructureSlice {
                    start: i.start,
                    slice: &mut i.slice[..],
                })
                .collect::<Vec<_>>();
            let mut slices = left_slices;
            slices.push(StructureSlice {
                start: slice.start,
                slice: left,
            });
            slices.push(StructureSlice {
                start: idx,
                slice: right,
            });
            slices.extend(right_slices.iter_mut().map(|i| StructureSlice {
                start: i.start,
                slice: &mut i.slice[..],
            }));
            Ok((center.dynamic.as_deref_mut(), StructureDynIter(slices)))
        } else {
            js_err!("Strucutre slices out of range")
        }
    }

    /// Accessor without generation checking.
    #[allow(dead_code)]
    pub(crate) fn get_at(&self, idx: usize) -> Option<&StructureEntry> {
        self.0
            .iter()
            .find(|slice| slice.start <= idx && idx < slice.start + slice.slice.len())
            .and_then(|slice| slice.slice.get(idx - slice.start))
    }

    /// Mutable accessor without generation checking.
    #[allow(dead_code)]
    pub(crate) fn get_at_mut(&mut self, idx: usize) -> Option<&mut StructureEntry> {
        self.0
            .iter_mut()
            .find(|slice| slice.start <= idx && idx < slice.start + slice.slice.len())
            .and_then(|slice| slice.slice.get_mut(idx - slice.start))
    }

    /// Accessor with generation checking.
    #[allow(dead_code)]
    pub(crate) fn get(&self, id: StructureId) -> Option<&dyn Structure> {
        let idx = id.id as usize;
        self.0
            .iter()
            .find(|slice| slice.start <= idx && idx < slice.start + slice.slice.len())
            .and_then(|slice| {
                slice
                    .slice
                    .get(idx - slice.start)
                    .filter(|s| s.gen == id.gen)
                    .and_then(|s| s.dynamic.as_deref())
            })
    }

    /// Mutable accessor with generation checking.
    pub(crate) fn get_mut(&mut self, id: StructureId) -> Option<&mut (dyn Structure + '_)> {
        let idx = id.id as usize;
        self.0
            .iter_mut()
            .find(|slice| slice.start <= idx && idx < slice.start + slice.slice.len())
            .and_then(|slice| {
                slice
                    .slice
                    .get_mut(idx - slice.start)
                    .filter(|s| s.gen == id.gen)
                    .and_then(|s| s.dynamic.as_deref_mut().map(|s| s as &mut dyn Structure))
                // Interestingly, we need .map(|s| s as &mut dyn Structure) to compile.
                // .map(|s| s.dynamic.as_deref_mut())
            })
    }

    pub(crate) fn dyn_iter_id(&self) -> impl Iterator<Item = (StructureId, &dyn Structure)> + '_ {
        self.0
            .iter()
            .flat_map(move |slice| {
                let start = slice.start;
                slice
                    .slice
                    .iter()
                    .enumerate()
                    .map(move |(i, val)| (i + start, val))
            })
            .filter_map(|(id, val)| {
                Some((
                    StructureId {
                        id: id as u32,
                        gen: val.gen,
                    },
                    val.dynamic.as_deref()?,
                ))
            })
    }
}

impl<'a> DynIter for StructureDynIter<'a> {
    type Item = dyn Structure;
    fn dyn_iter(&self) -> Box<dyn Iterator<Item = &Self::Item> + '_> {
        Box::new(
            self.0
                .iter()
                .flat_map(|slice| slice.slice.iter().filter_map(|s| s.dynamic.as_deref())),
        )
    }
    fn as_dyn_iter(&self) -> &dyn DynIter<Item = Self::Item> {
        self
    }
}

impl<'a> DynIterMut for StructureDynIter<'a> {
    fn dyn_iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut Self::Item> + '_> {
        Box::new(self.0.iter_mut().flat_map(|slice| {
            slice
                .slice
                .iter_mut()
                .filter_map(|s| s.dynamic.as_deref_mut())
        }))
    }
}
