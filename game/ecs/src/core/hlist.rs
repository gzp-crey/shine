use std::marker::PhantomData;

pub trait HList: Sized {
    fn push<N>(self, item: N) -> HCons<N, Self> {
        HCons(item, self)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HNil;
impl HList for HNil {}

pub struct HCons<Head, Tail>(pub Head, pub Tail);

impl<Head, Tail> HList for HCons<Head, Tail> {}

impl<Head, Tail> HCons<Head, Tail> {
    #[inline(always)]
    pub fn get<T, Index>(&self) -> &T
    where
        Self: Find<T, Index>,
    {
        Find::get(self)
    }

    #[inline(always)]
    pub fn get_mut<T, Index>(&mut self) -> &mut T
    where
        Self: Find<T, Index>,
    {
        Find::get_mut(self)
    }
}

/// Helper to destructure a HList and find Type location - found case
#[allow(dead_code)]
pub enum HHere {}
/// Helper to destructure a HList and find Type location - work-to-be-done case
#[allow(dead_code)]
pub struct HThere<T>(std::marker::PhantomData<T>);

impl Default for HNil {
    fn default() -> Self {
        HNil
    }
}

impl<T: Default, Tail: Default + HList> Default for HCons<T, Tail> {
    fn default() -> Self {
        HCons(T::default(), Tail::default())
    }
}

/// Trait to find T in the HList.
/// If type is not found (or exists at multiple location) a cannot infer type for type
/// parameter `TypeLocation` error is generated.
pub trait Find<T, TypeLocation> {
    fn get(&self) -> &T;
    fn get_mut(&mut self) -> &mut T;
}

impl<T, Tail> Find<T, HHere> for HCons<T, Tail> {
    fn get(&self) -> &T {
        &self.0
    }
    fn get_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<Head, T, Tail, TailIndex> Find<T, HThere<TailIndex>> for HCons<Head, Tail>
where
    Tail: Find<T, TailIndex>,
{
    fn get(&self) -> &T {
        self.1.get()
    }
    fn get_mut(&mut self) -> &mut T {
        self.1.get_mut()
    }
}

/// Returns an `HList` based on the values passed in.
#[macro_export]
macro_rules! hlist {
    () => { $crate::core::hlist::HNil };
    (...$rest:expr) => { $rest };
    ($a:expr) => { $crate::hlist![$a,] };
    ($a:expr, $($tok:tt)*) => {
        $crate::core::hlist::HCons {
            head: $a,
            tail: $crate::hlist![$($tok)*],
        }
    };
}

/// Returns a type signature for an HList of the provided types
#[macro_export]
macro_rules! hlist_type {
    () => { $crate::core::hlist::HNil };
    (...$Rest:ty) => { $Rest };
    ($A:ty) => { $crate::hlist_type![$A,] };
    ($A:ty, $($tok:tt)*) => {
        $crate::core::hlist::HCons<$A, $crate::hlist_type![$($tok)*]>
    };
}
