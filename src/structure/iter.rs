use super::{StructureBundle, StructureEntry, StructureId};
use crate::dyn_iter::{DynIter, DynIterMut};
use smallvec::{smallvec, SmallVec};
use wasm_bindgen::prelude::*;

#[derive(Default)]
pub(crate) struct StructureSlice<'a> {
    start: usize,
    slice: &'a mut [StructureEntry],
}

impl<'a> StructureSlice<'a> {
    /// A "dirty" clone that takes mutable reference.
    /// Because it requires mutable reference to self, we cannot implement Clone trait.
    ///
    /// Conceptually, it sounds weird that you need a mutable reference in order to clone,
    /// but in this case what we need is the exclusivity, not the mutability, to ensure that
    /// our internal mutable slice would not have aliases.
    ///
    /// Lifetime annotation is still a bit weird, it should return StructureSlice<'a> since the
    /// underlying StructureEntry lifetime should not change by making a slice to it, but
    /// somehow it fails to compile if I do.
    fn clone(&mut self) -> StructureSlice {
        StructureSlice {
            start: self.start,
            slice: self.slice,
        }
    }
}

/// A structure that allow random access to structure array with possible gaps.
///
/// It uses a SmallVec of slices, which will put the slices inline into the struct and avoid heap allocation
/// up to 2 elements. Most of the time, we only need left and right slices, which are inlined.
/// In rare occasions we want more slices and it will fall back to heap allocation.
/// This design requires a little inconvenience in exchange. That is, explicitly dropping the StructureDynIter before
/// being able to access the structures pointed to, like the example below. It seems to have something to do with the SmallVec's drop check,
/// but I'm not sure.
///
/// ```ignore
/// fn a(structures: &mut [StructureEntry]) {
///     let (_, iter) = StructureDynIter::new(&mut structures);
///     drop(iter);
///     structures[0].dynamic.name();
/// }
/// ```
///
/// It can access internal object in O(n) where n is the number of slices, not the number of objects.
/// It is convenient when you want to have mutable reference to two elements in the array at the same time.
pub(crate) struct StructureDynIter<'a>(SmallVec<[StructureSlice<'a>; 2]>);

impl<'a> StructureDynIter<'a> {
    pub(crate) fn new_all(source: &'a mut [StructureEntry]) -> Self {
        Self(smallvec![StructureSlice {
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
            Self(smallvec![
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
    ) -> Result<(Option<&'b mut StructureBundle>, StructureDynIter<'b>), JsValue>
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
            if entry.gen != id.gen || entry.bundle.is_none() {
                return Ok((
                    None,
                    StructureDynIter(self.0.iter_mut().map(|i| i.clone()).collect()),
                ));
            }

            // [slice_0] [slice_1] .. [left..center..right] .. [slice_i+1] .. [slice_n]
            //   to
            // [slice_0] [slice_1] .. [left] [right] .. [slice_i+1] .. [slice_n]
            //    and  center
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
                .map(|i| i.clone())
                .collect::<SmallVec<_>>();
            let mut slices = left_slices;
            slices.push(StructureSlice {
                start: slice.start,
                slice: left,
            });
            slices.push(StructureSlice {
                start: idx,
                slice: right,
            });
            slices.extend(right_slices.iter_mut().map(|i| i.clone()));
            Ok((center.bundle.as_mut(), StructureDynIter(slices)))
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
    pub(crate) fn get(&self, id: StructureId) -> Option<&StructureBundle> {
        let idx = id.id as usize;
        self.0
            .iter()
            .find(|slice| slice.start <= idx && idx < slice.start + slice.slice.len())
            .and_then(|slice| {
                slice
                    .slice
                    .get(idx - slice.start)
                    .filter(|s| s.gen == id.gen)
                    .and_then(|s| s.bundle.as_ref())
            })
    }

    /// Mutable accessor with generation checking.
    pub(crate) fn get_mut(&mut self, id: StructureId) -> Option<&mut StructureBundle> {
        let idx = id.id as usize;
        self.0
            .iter_mut()
            .find(|slice| slice.start <= idx && idx < slice.start + slice.slice.len())
            .and_then(|slice| {
                slice
                    .slice
                    .get_mut(idx - slice.start)
                    .filter(|s| s.gen == id.gen)
                    .and_then(|s| s.bundle.as_mut())
                // Interestingly, we need .map(|s| s as &mut dyn Structure) to compile.
                // .map(|s| s.dynamic.as_deref_mut())
            })
    }

    pub(crate) fn dyn_iter_id(&self) -> impl Iterator<Item = (StructureId, &StructureBundle)> + '_ {
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
                    val.bundle.as_ref()?,
                ))
            })
    }
}

impl<'a> DynIter for StructureDynIter<'a> {
    type Item = StructureBundle;
    fn dyn_iter(&self) -> Box<dyn Iterator<Item = &Self::Item> + '_> {
        Box::new(
            self.0
                .iter()
                .flat_map(|slice| slice.slice.iter().filter_map(|s| s.bundle.as_ref())),
        )
    }
    fn as_dyn_iter(&self) -> &dyn DynIter<Item = Self::Item> {
        self
    }
}

impl<'a> DynIterMut for StructureDynIter<'a> {
    fn dyn_iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut Self::Item> + '_> {
        Box::new(
            self.0
                .iter_mut()
                .flat_map(|slice| slice.slice.iter_mut().filter_map(|s| s.bundle.as_mut())),
        )
    }
}
