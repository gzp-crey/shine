use crate::{
    core::hlist::HFind,
    hlist_type,
    resources::{Resource, Resources},
    scheduler::{
        FetchResource, IntoResourceClaim, IntoSystem, MultiResClaim, MultiResMutClaim, ResourceClaims, ResourceQuery,
        System, SystemName, TaskGroup,
    },
    ECSError,
};
use std::{any, borrow::Cow};

/// Helper trait to set the tags for shared tagged resource claims
/// #Example
/// ```
/// # use shine_ecs::{ECSError, resources::ResourceId, scheduler::*};
/// fn some_system(r1: MultiRes<u8>, r2: MultiRes<u16>) -> Result<TaskGroup, ECSError> {
///    Ok(TaskGroup::default())
/// }
///
/// let mut tg = TaskGroup::default();
/// tg.add_system(
///    some_system.into_system()
///        .claim_res::<u8, _>(|claim| claim.append_ids(Some(ResourceId::Global)))
///        .try_claim_res::<u16, _>(|claim| claim.try_append_tags(&["tag"])).unwrap(),
/// );
/// ```
pub trait WithMultiRes<C, HIndex> {
    fn claim_res<T, F>(self, claim: F) -> Self
    where
        Self: Sized,
        T: Resource,
        C: HFind<MultiResClaim<T>, HIndex>,
        F: FnMut(&mut MultiResClaim<T>);

    fn try_claim_res<T, F>(self, claim: F) -> Result<Self, ECSError>
    where
        Self: Sized,
        C: HFind<MultiResClaim<T>, HIndex>,
        T: Resource,
        F: FnMut(&mut MultiResClaim<T>) -> Result<(), ECSError>;
}

/// Helper trait to set the tags for unique tagged resource claims
/// #Example
/// ```
/// # use shine_ecs::{ECSError, resources::ResourceId, scheduler::*};
/// fn some_system(r1: MultiResMut<u8>, r2: MultiResMut<u16>) -> Result<TaskGroup, ECSError> {
///    Ok(TaskGroup::default())
/// }
///
/// let mut tg = TaskGroup::default();
/// tg.add_system(
///    some_system.into_system()
///        .claim_res_mut::<u8, _>(|claim| claim.append_ids(Some(ResourceId::Global)))
///        .try_claim_res_mut::<u16, _>(|claim| claim.try_append_tags(&["tag"])).unwrap(),
/// );
/// ```
pub trait WithMultiResMut<C, Index> {
    fn claim_res_mut<T, F>(self, claim: F) -> Self
    where
        Self: Sized,
        T: Resource,
        C: HFind<MultiResMutClaim<T>, Index>,
        F: FnMut(&mut MultiResMutClaim<T>);

    fn try_claim_res_mut<T, F>(self, claim: F) -> Result<Self, ECSError>
    where
        Self: Sized,
        C: HFind<MultiResMutClaim<T>, Index>,
        T: Resource,
        F: FnMut(&mut MultiResMutClaim<T>) -> Result<(), ECSError>;
}

macro_rules! impl_into_system {
    ($sys: ident, ($($resource: ident),*)) => {
        impl<Func, $($resource,)*> IntoSystem<($($resource,)*)> for Func
        where
            Func:
                FnMut($($resource,)*) -> Result<TaskGroup, ECSError> +
                FnMut($(<<$resource as ResourceQuery>::Fetch as FetchResource<'_, <$resource as ResourceQuery>::Claim>>::Item,)*) -> Result<TaskGroup, ECSError>
                + Send + Sync + 'static,
            $($resource: ResourceQuery,)*
        {
            type System = $sys<Func, $($resource,)*>;

            fn into_system(self) -> Self::System {
                $sys {
                    debug_name: any::type_name::<Func>().into(),
                    name: None,
                    claims: Default::default(),
                    resource_claims: None,
                    func: self
                }
            }
        }

        /// System created from a function
        pub struct $sys<Func, $($resource,)*>
        where
            Func:
                FnMut( $($resource,)*) -> Result<TaskGroup, ECSError> +
                FnMut( $(<<$resource as ResourceQuery>::Fetch as FetchResource<'_, <$resource as ResourceQuery>::Claim>>::Item,)*) -> Result<TaskGroup, ECSError> +
                Send + Sync + 'static,
            $($resource: ResourceQuery,)*
        {
            debug_name: Cow<'static, str>,
            name: Option<SystemName>,
            claims: hlist_type![$(<$resource as ResourceQuery>::Claim,)*],
            resource_claims: Option<ResourceClaims>,
            func: Func,
        }

        impl<Func, $($resource,)*> $sys<Func, $($resource,)*>
        where
            Func:
                FnMut( $($resource,)*) -> Result<TaskGroup, ECSError> +
                FnMut( $(<<$resource as ResourceQuery>::Fetch as FetchResource<'_, <$resource as ResourceQuery>::Claim>>::Item,)*) -> Result<TaskGroup, ECSError> +
                Send + Sync + 'static,
            $($resource: ResourceQuery,)*
        {
            pub fn with_name(mut self, name: Option<SystemName>) -> Self {
                self.name = name;
                self
            }
        }

        impl<HIndex, Func, $($resource,)*> WithMultiRes<hlist_type![$(<$resource as ResourceQuery>::Claim,)*], HIndex>
            for $sys<Func, $($resource,)*>
        where
            Func:
                FnMut( $($resource,)*) -> Result<TaskGroup, ECSError> +
                FnMut( $(<<$resource as ResourceQuery>::Fetch as FetchResource<'_, <$resource as ResourceQuery>::Claim>>::Item,)*) -> Result<TaskGroup, ECSError> +
                Send + Sync + 'static,
            $($resource: ResourceQuery,)*
        {
            fn claim_res<T, F>(mut self, mut claim: F) -> Self
            where
                Self: Sized,
                T: Resource,
                hlist_type![$(<$resource as ResourceQuery>::Claim,)*]: HFind<MultiResClaim<T>, HIndex>,
                F: FnMut(&mut MultiResClaim<T>),
            {
                claim(self.claims.get_mut());
                self.resource_claims = None;
                self
            }

            fn try_claim_res<T, F>(mut self, mut claim: F) -> Result<Self, ECSError>
            where
                Self: Sized,
                T: Resource,
                hlist_type![$(<$resource as ResourceQuery>::Claim,)*]: HFind<MultiResClaim<T>, HIndex>,
                F: FnMut(&mut MultiResClaim<T>) -> Result<(), ECSError>,
            {
                claim(self.claims.get_mut())?;
                self.resource_claims = None;
                Ok(self)
            }
        }

        impl<HIndex, Func, $($resource,)*> WithMultiResMut<hlist_type![$(<$resource as ResourceQuery>::Claim,)*], HIndex>
            for $sys<Func, $($resource,)*>
        where
            Func:
                FnMut( $($resource,)*) -> Result<TaskGroup, ECSError> +
                FnMut( $(<<$resource as ResourceQuery>::Fetch as FetchResource<'_, <$resource as ResourceQuery>::Claim>>::Item,)*) -> Result<TaskGroup, ECSError> +
                Send + Sync + 'static,
            $($resource: ResourceQuery,)*
        {
            fn claim_res_mut<T, F>(mut self, mut claim: F) -> Self
            where
                Self: Sized,
                T: Resource,
                hlist_type![$(<$resource as ResourceQuery>::Claim,)*]: HFind<MultiResMutClaim<T>, HIndex>,
                F: FnMut(&mut MultiResMutClaim<T>),
            {
                claim(self.claims.get_mut());
                self.resource_claims = None;
                self
            }

            fn try_claim_res_mut<T, F>(mut self, mut claim: F) -> Result<Self, ECSError>
            where
                Self: Sized,
                T: Resource,
                hlist_type![$(<$resource as ResourceQuery>::Claim,)*]: HFind<MultiResMutClaim<T>, HIndex>,
                F: FnMut(&mut MultiResMutClaim<T>) -> Result<(), ECSError>,
            {
                claim(self.claims.get_mut())?;
                self.resource_claims = None;
                Ok(self)
            }
        }

        #[allow(unused_variables)]
        #[allow(unused_mut)]
        #[allow(non_snake_case)]
        impl<Func, $($resource,)*> System for $sys<Func, $($resource,)*>
        where
            Func:
                FnMut( $($resource,)*) -> Result<TaskGroup, ECSError> +
                FnMut( $(<<$resource as ResourceQuery>::Fetch as FetchResource<'_, <$resource as ResourceQuery>::Claim>>::Item,)*) -> Result<TaskGroup, ECSError> +
                Send + Sync + 'static,
            $($resource: ResourceQuery,)*
        {
            fn debug_name(&self) -> &str {
                &self.debug_name
            }

            fn name(&self) -> Option<&SystemName> {
                self.name.as_ref()
            }

            fn resource_claims(&mut self) -> Result<&ResourceClaims, ECSError> {
                let claims = &self.claims;
                Ok(self.resource_claims.get_or_insert_with(|| {
                    let mut resource_claims = ResourceClaims::default();
                    $(resource_claims.add_claim({
                        let $resource : &<$resource as ResourceQuery>::Claim = claims.get();
                        $resource.to_claim()
                    });)*
                    resource_claims
                }))
            }

            fn run(&mut self, resources: &Resources) -> Result<TaskGroup, ECSError> {
                $(
                    let mut $resource = <<$resource as ResourceQuery>::Fetch as FetchResource<'_, <$resource as ResourceQuery>::Claim>>::fetch(
                        resources,
                        {
                            let $resource : &<$resource as ResourceQuery>::Claim = self.claims.get();
                            $resource
                        }
                    )?;
                )*
                (self.func)($($resource,)*)
            }
        }
    }
}

macro_rules! impl_into_systems {
    ($sys: ident, $($resource: ident),*) => {
        impl_into_system!($sys, ($($resource),*));
    };
}

impl_into_systems!(FNSystem0,);
impl_into_systems!(FNSystem1, Ra);
impl_into_systems!(FNSystem2, Ra, Rb);
impl_into_systems!(FNSystem3, Ra, Rb, Rc);
impl_into_systems!(FNSystem4, Ra, Rb, Rc, Rd);
impl_into_systems!(FNSystem5, Ra, Rb, Rc, Rd, Re);
impl_into_systems!(FNSystem6, Ra, Rb, Rc, Rd, Re, Rf);
impl_into_systems!(FNSystem7, Ra, Rb, Rc, Rd, Re, Rf, Rg);
impl_into_systems!(FNSystem8, Ra, Rb, Rc, Rd, Re, Rf, Rg, Rh);
impl_into_systems!(FNSystem9, Ra, Rb, Rc, Rd, Re, Rf, Rg, Rh, Ri);
impl_into_systems!(FNSystem10, Ra, Rb, Rc, Rd, Re, Rf, Rg, Rh, Ri, Rj);
