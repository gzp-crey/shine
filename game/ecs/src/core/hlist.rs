// A stripped version of a the more genera HList implementation from https://github.com/lloydmeta/frunk/tree/master/core.

use std::marker::PhantomData;

pub trait HList: Sized {
    fn push<N>(self, item: N) -> HCons<N, Self> {
        HCons(item, self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HNil;
impl HList for HNil {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HCons<Head, Tail>(pub Head, pub Tail);

impl<Head, Tail> HList for HCons<Head, Tail> {}

impl<Head, Tail> HCons<Head, Tail> {
    #[inline(always)]
    pub fn get<T, Index>(&self) -> &T
    where
        Self: HFind<T, Index>,
    {
        HFind::get(self)
    }

    #[inline(always)]
    pub fn get_mut<T, Index>(&mut self) -> &mut T
    where
        Self: HFind<T, Index>,
    {
        HFind::get_mut(self)
    }

    /// Return an `HList` where the contents are references
    /// to the original `HList` on which this method was called.
    ///
    /// # Examples
    ///
    /// ```
    /// # use shine_ecs::{hlist, core::hlist::*};
    /// assert_eq!(hlist![].to_ref(), hlist![]);
    /// assert_eq!(hlist![1, true].to_ref(), hlist![&1, &true]);
    /// ```
    #[inline(always)]
    pub fn to_ref<'a>(&'a mut self) -> <Self as ToRef<'a>>::Output
    where
        Self: ToRef<'a>,
    {
        ToRef::to_ref(self)
    }

    /// Return an `HList` where the contents are mutable references to the original `HList` on which
    /// this method was called. Using the `get`, `get_mut` functions usually won't provides the required
    /// behavior and won't allow to borrow mutiple type ath the same time.
    /// (`to_mut().pluck::<T,_>()` provides a slicing solution)
    ///
    /// # Examples
    ///
    /// ```
    /// # use shine_ecs::{hlist, core::hlist::*};
    /// assert_eq!(hlist![].to_mut(), hlist![]);
    /// assert_eq!(hlist![1, true].to_mut(), hlist![&mut 1, &mut true]);
    /// ```
    #[inline(always)]
    pub fn to_mut<'a>(&'a mut self) -> <Self as ToMut<'a>>::Output
    where
        Self: ToMut<'a>,
    {
        ToMut::to_mut(self)
    }

    /// Remove an element by type from an HList.
    ///
    /// The remaining elements are returned along with it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use shine_ecs::{hlist, core::hlist::*};
    /// let list = hlist![1, "hello", true, 42f32];
    /// let (b, list): (bool, _) = list.pluck();
    /// assert!(b);
    /// let (s, list) = list.pluck::<i32, _>();
    /// assert_eq!(list, hlist!["hello", 42.0])
    /// ```
    #[inline(always)]
    pub fn pluck<T, Index>(self) -> (T, <Self as Plucker<T, Index>>::Remainder)
    where
        Self: Plucker<T, Index>,
    {
        Plucker::pluck(self)
    }
}

/// Helper to destructure a HList and find Type location - found case
#[allow(dead_code)]
pub enum HHere {}
/// Helper to destructure a HList and find Type location - work-to-be-done case
#[allow(dead_code)]
pub struct HThere<T>(PhantomData<T>);

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
pub trait HFind<T, TypeLocation> {
    fn get(&self) -> &T;
    fn get_mut(&mut self) -> &mut T;
}

impl<T, Tail> HFind<T, HHere> for HCons<T, Tail> {
    fn get(&self) -> &T {
        &self.0
    }
    fn get_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<Head, T, Tail, TailIndex> HFind<T, HThere<TailIndex>> for HCons<Head, Tail>
where
    Tail: HFind<T, TailIndex>,
{
    fn get(&self) -> &T {
        self.1.get()
    }
    fn get_mut(&mut self) -> &mut T {
        self.1.get_mut()
    }
}

pub trait Plucker<Target, Index> {
    type Remainder;

    /// Remove an element by type from an HList.
    fn pluck(self) -> (Target, Self::Remainder);
}

/// Implementation when the pluck target is in head
impl<T, Tail> Plucker<T, HHere> for HCons<T, Tail> {
    type Remainder = Tail;

    fn pluck(self) -> (T, Self::Remainder) {
        (self.0, self.1)
    }
}

/// Implementation when the pluck target is in the tail
impl<Head, Tail, FromTail, TailIndex> Plucker<FromTail, HThere<TailIndex>> for HCons<Head, Tail>
where
    Tail: Plucker<FromTail, TailIndex>,
{
    type Remainder = HCons<Head, <Tail as Plucker<FromTail, TailIndex>>::Remainder>;

    fn pluck(self) -> (FromTail, Self::Remainder) {
        let (target, tail_remainder): (FromTail, <Tail as Plucker<FromTail, TailIndex>>::Remainder) =
            <Tail as Plucker<FromTail, TailIndex>>::pluck(self.1);
        (target, HCons(self.0, tail_remainder))
    }
}

/// Helper trait to implement to_ref
pub trait ToRef<'a> {
    type Output;

    fn to_ref(&'a self) -> Self::Output;
}

impl<'a> ToRef<'a> for HNil {
    type Output = HNil;

    #[inline(always)]
    fn to_ref(&'a self) -> Self::Output {
        HNil
    }
}

impl<'a, H, Tail> ToRef<'a> for HCons<H, Tail>
where
    H: 'a,
    Tail: ToRef<'a>,
{
    type Output = HCons<&'a H, <Tail as ToRef<'a>>::Output>;

    #[inline(always)]
    fn to_ref(&'a self) -> Self::Output {
        HCons(&self.0, (&self.1).to_ref())
    }
}

/// Helper trait to implement to_mut
pub trait ToMut<'a> {
    type Output;

    fn to_mut(&'a mut self) -> Self::Output;
}

impl<'a> ToMut<'a> for HNil {
    type Output = HNil;

    #[inline(always)]
    fn to_mut(&'a mut self) -> Self::Output {
        HNil
    }
}

impl<'a, H, Tail> ToMut<'a> for HCons<H, Tail>
where
    H: 'a,
    Tail: ToMut<'a>,
{
    type Output = HCons<&'a mut H, <Tail as ToMut<'a>>::Output>;

    #[inline(always)]
    fn to_mut(&'a mut self) -> Self::Output {
        HCons(&mut self.0, self.1.to_mut())
    }
}

/// Returns an `HList` based on the values passed in.
#[macro_export]
macro_rules! hlist {
    () => { $crate::core::hlist::HNil };
    (...$rest:expr) => { $rest };
    ($a:expr) => { $crate::hlist![$a,] };
    ($a:expr, $($tok:tt)*) => {
        $crate::core::hlist::HCons($a,$crate::hlist![$($tok)*])
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
