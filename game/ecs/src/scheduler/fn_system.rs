use crate::{
    core::hlist::HFind,
    hlist_type,
    resources::{Resource, Resources},
    scheduler::{
        FetchResource, IntoResourceClaim, IntoSystem, IntoSystemBuilder, MultiResClaim, MultiResMutClaim,
        ResourceClaims, ResourceQuery, System, SystemName, TaskGroup,
    },
    ECSError,
};
use std::{any, borrow::Cow, marker::PhantomData};

/// Create a system from a function
pub struct FnSystemBuilder<Func, C, R> {
    func: Func,
    name: Option<SystemName>,
    dependencies: Vec<SystemName>,
    claims: C,
    _phantom: PhantomData<R>,
}

impl<Func, C, R> FnSystemBuilder<Func, C, R> {
    #[must_use]
    pub fn with_name(mut self, name: Option<SystemName>) -> Self {
        self.name = name;
        self
    }

    #[must_use]
    pub fn with_dependencies(mut self, names: &[SystemName]) -> Self {
        self.dependencies.extend(names.iter().cloned());
        self
    }

    #[must_use]
    pub fn with_claim<Claim, Index>(mut self, claim: Claim) -> Self
    where
        C: HFind<Claim, Index>,
    {
        *self.claims.get_mut() = claim;
        self
    }
}

/// Helper trait to set the tags for shared tagged resource claims
/// #Example
/// ```
/// use shine_ecs::scheduler::*;
/// fn some_system(r1: MultiRes<u8>, r2: MultiRes<u16>) {}
///
/// let mut tg = TaskGroup::default();
/// tg.add(
///    some_system.system()
///        .try_claim_res::<u8, _>(|claim| claim.try_append_tags(&["one", "two"]))
///        .unwrap()
///        .try_claim_res::<u16, _>(|claim| claim.try_append_tags(&["three"]))
///        .unwrap(),
/// )
/// .unwrap();
/// ```
pub trait WithMultiRes<C, HIndex> {
    fn claim_res<T, F>(self, claim: F) -> Self
    where
        Self: Sized,
        C: HFind<MultiResClaim<T>, HIndex>,
        T: Resource,
        F: FnMut(&mut MultiResClaim<T>);

    fn try_claim_res<T, F>(self, claim: F) -> Result<Self, ECSError>
    where
        Self: Sized,
        C: HFind<MultiResClaim<T>, HIndex>,
        T: Resource,
        F: FnMut(&mut MultiResClaim<T>) -> Result<(), ECSError>;
}

impl<Func, C, R, HIndex> WithMultiRes<C, HIndex> for FnSystemBuilder<Func, C, R> {
    fn claim_res<T, F>(mut self, mut claim: F) -> Self
    where
        Self: Sized,
        C: HFind<MultiResClaim<T>, HIndex>,
        T: Resource,
        F: FnMut(&mut MultiResClaim<T>),
    {
        claim(self.claims.get_mut());
        self
    }

    fn try_claim_res<T, F>(mut self, mut claim: F) -> Result<Self, ECSError>
    where
        Self: Sized,
        C: HFind<MultiResClaim<T>, HIndex>,
        T: Resource,
        F: FnMut(&mut MultiResClaim<T>) -> Result<(), ECSError>,
    {
        claim(self.claims.get_mut())?;
        Ok(self)
    }
}

/// Helper trait to set the tags for unique tagged resource claims
/// #Example
/// ```
/// use shine_ecs::scheduler::*;
/// fn some_system(r1: MultiResMut<u8>, r2: MultiResMut<u16>) {}
///
/// let mut tg = TaskGroup::default();
/// tg.add(
///    some_system.system()
///        .try_claim_res_mut::<u8, _>(|claim| claim.try_append_tags(&["one", "two"]))
///        .unwrap()
///        .try_claim_res_mut::<u16, _>(|claim| claim.try_append_tags(&["three"]))
///        .unwrap(),
/// )
/// .unwrap();
/// ```
pub trait WithMultiResMut<C, Index> {
    fn claim_res_mut<T: Resource, F: FnMut(&mut MultiResMutClaim<T>)>(self, claim: F) -> Self
    where
        Self: Sized,
        C: HFind<MultiResMutClaim<T>, Index>;

    fn try_claim_res_mut<T, F>(self, claim: F) -> Result<Self, ECSError>
    where
        Self: Sized,
        C: HFind<MultiResMutClaim<T>, Index>,
        T: Resource,
        F: FnMut(&mut MultiResMutClaim<T>) -> Result<(), ECSError>;
}

impl<Func, C, R, Index> WithMultiResMut<C, Index> for FnSystemBuilder<Func, C, R> {
    fn claim_res_mut<T, F>(mut self, mut claim: F) -> Self
    where
        Self: Sized,
        C: HFind<MultiResMutClaim<T>, Index>,
        T: Resource,
        F: FnMut(&mut MultiResMutClaim<T>),
    {
        claim(self.claims.get_mut());
        self
    }

    fn try_claim_res_mut<T, F>(mut self, mut claim: F) -> Result<Self, ECSError>
    where
        Self: Sized,
        C: HFind<MultiResMutClaim<T>, Index>,
        T: Resource,
        F: FnMut(&mut MultiResMutClaim<T>) -> Result<(), ECSError>,
    {
        claim(self.claims.get_mut())?;
        Ok(self)
    }
}

pub struct FnSystem<Func, Claims>
where
    Func: FnMut(&Resources, &Claims) -> Result<(), ECSError>,
{
    debug_name: Cow<'static, str>,
    name: Option<SystemName>,
    claims: Claims,
    resource_claims: ResourceClaims,
    func: Func,
}

impl<Func, Claims> System for FnSystem<Func, Claims>
where
    Func: FnMut(&Resources, &Claims) -> Result<(), ECSError> + Send + Sync,
    Claims: 'static + Send + Sync,
{
    fn debug_name(&self) -> &str {
        &self.debug_name
    }

    fn name(&self) -> Option<&SystemName> {
        self.name.as_ref()
    }

    fn resource_claims(&self) -> &ResourceClaims {
        &self.resource_claims
    }

    fn run(&mut self, resources: &Resources) -> Result<TaskGroup, ECSError> {
        log::trace!("Running system [{:?}] - {:?}", self.name, self.debug_name());
        (self.func)(resources, &self.claims)?;
        Ok(TaskGroup::default())
    }
}

macro_rules! fn_call {
    ($func:ident, ($($resource: ident),*)) => {
        $func($($resource,)*)
    };
}

macro_rules! impl_into_system {
    (($($resource: ident),*)) => {
        impl<Func, $($resource,)*> IntoSystemBuilder<($($resource,)*)> for Func
        where
            Func:
                FnMut($($resource,)*) +
                FnMut($(<<$resource as ResourceQuery>::Fetch as FetchResource<'_, <$resource as ResourceQuery>::Claim>>::Item,)*)
                + Send + Sync + 'static,
            $($resource: ResourceQuery,)*
        {
            type Builder = FnSystemBuilder<Func,
                             hlist_type![$(<$resource as ResourceQuery>::Claim,)*],
                             ($($resource,)*)>;

            fn system(self) -> Self::Builder {
                FnSystemBuilder {
                    name: None,
                    func: self,
                    dependencies: Default::default(),
                    _phantom: PhantomData,
                    claims: Default::default(),
                }
            }
        }

        impl<Func, $($resource,)*> IntoSystem<($($resource,)*)>
            for FnSystemBuilder<
                    Func,
                    hlist_type![$(<$resource as ResourceQuery>::Claim,)*],
                    ($($resource,)*)>
        where
            Func:
                FnMut( $($resource,)*) +
                FnMut( $(<<$resource as ResourceQuery>::Fetch as FetchResource<'_, <$resource as ResourceQuery>::Claim>>::Item,)*) +
                Send + Sync + 'static,
            $($resource: ResourceQuery,)*
        {
            #[allow(unused_mut)]
            #[allow(unused_variables)]
            #[allow(non_snake_case)]
            fn into_system(self) -> Result<Box<dyn System>, ECSError> {
                let FnSystemBuilder{ mut func, name, claims, .. } = self;
                let debug_name = any::type_name::<Func>().into();
                let mut resource_claims = ResourceClaims::default();

                $(resource_claims.add_claim({
                        let $resource : &<$resource as ResourceQuery>::Claim = claims.get();
                        $resource.to_claim()
                    });)*

                Ok(Box::new(FnSystem {
                    name,
                    debug_name,
                    resource_claims,
                    claims,
                    func: move |resources, claims| {
                        $(
                            let mut $resource = <<$resource as ResourceQuery>::Fetch as FetchResource<'_, <$resource as ResourceQuery>::Claim>>::fetch(
                                resources,
                                {
                                    let $resource : &<$resource as ResourceQuery>::Claim = claims.get();
                                    $resource
                                }
                            )?;
                        )*
                        fn_call!(func, ($($resource),*));
                        Ok(())
                    }
                }))
            }
        }
    }
}

macro_rules! impl_into_systems {
    ($($resource: ident),*) => {
        impl_into_system!(($($resource),*));
    };
}

impl_into_systems!();
impl_into_systems!(Ra);
impl_into_systems!(Ra, Rb);
impl_into_systems!(Ra, Rb, Rc);
impl_into_systems!(Ra, Rb, Rc, Rd);
impl_into_systems!(Ra, Rb, Rc, Rd, Re);
impl_into_systems!(Ra, Rb, Rc, Rd, Re, Rf);
impl_into_systems!(Ra, Rb, Rc, Rd, Re, Rf, Rg);
impl_into_systems!(Ra, Rb, Rc, Rd, Re, Rf, Rg, Rh);
impl_into_systems!(Ra, Rb, Rc, Rd, Re, Rf, Rg, Rh, Ri);
impl_into_systems!(Ra, Rb, Rc, Rd, Re, Rf, Rg, Rh, Ri, Rj);
