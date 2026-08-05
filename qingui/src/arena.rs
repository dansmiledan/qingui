use alloc::vec::Vec;

/// Stable handle to an object in an [`Arena`]: a slot index paired with a generation
/// counter so stale handles never access reused slots.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct ObjRef {
    /// Slot index in the arena.
    pub index: u32,
    /// Generation of the slot, incremented on every removal.
    pub generation: u32,
}

struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

/// Generational slot arena: stores objects in stable, index-based slots and reuses
/// freed slots, detecting stale references via generation counters.
pub struct Arena<T> {
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
}

impl<T> Arena<T> {
    /// Creates an empty arena.
    pub fn new() -> Self {
        Self { slots: Vec::new(), free: Vec::new() }
    }
    /// Inserts a value and returns a stable handle to it, reusing a freed slot if available.
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
    /// Returns an immutable reference to the value behind `r`, or `None` if the
    /// handle is stale or the slot is empty.
    pub fn get(&self, r: ObjRef) -> Option<&T> {
        self.slots
            .get(r.index as usize)
            .filter(|s| s.generation == r.generation)
            .and_then(|s| s.value.as_ref())
    }
    /// Returns a mutable reference to the value behind `r`, or `None` if the
    /// handle is stale or the slot is empty.
    pub fn get_mut(&mut self, r: ObjRef) -> Option<&mut T> {
        self.slots
            .get_mut(r.index as usize)
            .filter(|s| s.generation == r.generation)
            .and_then(|s| s.value.as_mut())
    }
    /// Removes the value behind `r`, returns it, and frees the slot. Returns
    /// `None` if the handle is stale or the slot is already empty.
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
    /// Returns `true` if the handle `r` currently points to a live value.
    pub fn contains(&self, r: ObjRef) -> bool {
        self.get(r).is_some()
    }
}
