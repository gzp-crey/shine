use crate::{
    core::hlist::HFind,
    hlist_type,
    resources::{Resource, Resources},
    scheduler::{
        FetchResource, IntoResourceClaim, IntoSystem, IntoSystemBuilder, ResourceClaims, ResourceQuery, System,
        SystemGroup, SystemName, TagResClaim, TagResMutClaim,
    },
    ECSError,
};
use std::{any, borrow::Cow, convert::TryFrom, marker::PhantomData};

/// Create a system from a function
pub struct FnSystemBuilder<Func, C, R> {
    func: Func,
    name: Option<SystemName>,
    dependencies: Vec<SystemName>,
    claims: C,
    _phantom: std::marker::PhantomData<R>,
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
pub trait WithTagRes<C, HIndex> {
    fn with_tag<T: Resource>(self, claim: &[&str]) -> Self
    where
        Self: Sized,
        C: HFind<TagResClaim<T>, HIndex>;
}

impl<Func, C, R, HIndex> WithTagRes<C, HIndex> for FnSystemBuilder<Func, C, R> {
    fn with_tag<T: Resource>(mut self, claim: &[&str]) -> Self
    where
        Self: Sized,
        C: HFind<TagResClaim<T>, HIndex>,
    {
        *self.claims.get_mut() = TagResClaim::<T>::try_from(claim).unwrap();
        self
    }
}

/// Helper trait to set the tags for unique tagged resource claims
pub trait WithTagResMut<C, Index> {
    fn with_tag_mut<T: Resource>(self, claim: &[&str]) -> Self
    where
        Self: Sized,
        C: HFind<TagResMutClaim<T>, Index>;
}

impl<Func, C, R, Index> WithTagResMut<C, Index> for FnSystemBuilder<Func, C, R> {
    fn with_tag_mut<T: Resource>(mut self, claim: &[&str]) -> Self
    where
        Self: Sized,
        C: HFind<TagResMutClaim<T>, Index>,
    {
        *self.claims.get_mut() = TagResMutClaim::<T>::try_from(claim).unwrap();
        self
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

    fn name(&self) -> &Option<SystemName> {
        &self.name
    }

    fn resource_claims(&self) -> &ResourceClaims {
        &self.resource_claims
    }

    fn run(&mut self, resources: &Resources) -> Result<SystemGroup, ECSError> {
        log::trace!("Running system [{:?}] - {:?}", self.name, self.debug_name());
        (self.func)(resources, &self.claims)?;
        Ok(SystemGroup::default())
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
                    _phantom: std::marker::PhantomData,
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
                        $resource.into_claim()
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
