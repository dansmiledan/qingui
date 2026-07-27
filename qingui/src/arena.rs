use alloc::vec::Vec;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct ObjRef {
    pub index: u32,
    pub generation: u32,
}

struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

pub struct Arena<T> {
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self { slots: Vec::new(), free: Vec::new() }
    }
    pub fn insert(&mut self, v: T) -> ObjRef {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.value = Some(v);
            ObjRef { index, generation: slot.generation }
        } else {
            self.slots.push(Slot { generation: 0, value: Some(v) });
            ObjRef { index: (self.slots.len() - 1) as u32, generation: 0 }
        }
    }
    pub fn get(&self, r: ObjRef) -> Option<&T> {
        self.slots
            .get(r.index as usize)
            .filter(|s| s.generation == r.generation)
            .and_then(|s| s.value.as_ref())
    }
    pub fn get_mut(&mut self, r: ObjRef) -> Option<&mut T> {
        self.slots
            .get_mut(r.index as usize)
            .filter(|s| s.generation == r.generation)
            .and_then(|s| s.value.as_mut())
    }
    pub fn remove(&mut self, r: ObjRef) -> Option<T> {
        let slot = self.slots.get_mut(r.index as usize)?;
        if slot.generation != r.generation {
            return None;
        }
        let v = slot.value.take()?;
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(r.index);
        Some(v)
    }
    pub fn contains(&self, r: ObjRef) -> bool {
        self.get(r).is_some()
    }
}
